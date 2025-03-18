use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{mpsc, Arc, LazyLock, RwLock},
    thread,
    time::{Duration, Instant},
};

use notify_debouncer_full::{new_debouncer, notify};
use shaderc::{Compiler, CompileOptions, ResolvedInclude, ShaderKind};
use vulkano::{
    device::Device,
    shader::{ShaderModule, ShaderModuleCreateInfo},
};

const DEBOUNCE_TIME: Duration = Duration::from_millis(500);
const MAX_INCLUDE_DEPTH: usize = 16;

static COMPILE_THREAD: LazyLock<mpsc::Sender<Arc<HotShader>>> = LazyLock::new(|| {
    let (tx, rx) = mpsc::channel::<Arc<HotShader>>();
    thread::spawn(move || {
        while let Ok(shader) = rx.recv() {
            if let Err(err) = shader.compile_code() {
                match &shader.path {
                    Some(path) => log::error!("Error compiling shader {}: {err:#}", path.display()),
                    None => log::error!("Error compiling shader: {err:#}"),
                }
            }
        }
    });
    tx
});

pub fn watch_shaders<S: IntoIterator<Item = Arc<HotShader>>>(shaders: S) {
    let shaders_by_path = shaders.into_iter()
        .filter_map(|shader| {
            shader.path.as_ref()
                .and_then(|path| fs::canonicalize(path).ok())
                .map(|path| (path, shader))
        })
        .collect::<HashMap<_, _>>();

    thread::spawn(move || {
        let (tx, rx) = mpsc::channel();
        let mut debouncer = match new_debouncer(DEBOUNCE_TIME, None, tx) {
            Ok(debouncer) => debouncer,
            Err(err) => {
                log::error!("failed to create file watcher: {err}");
                return;
            }
        };
        let dirs_to_watch = shaders_by_path.keys()
            .filter_map(|path| path.parent())
            .collect::<HashSet<_>>();
        for path in dirs_to_watch {
            if let Err(err) = debouncer.watch(path, notify::RecursiveMode::Recursive) {
                log::error!("failed to watch {}: {err}", path.display());
            } else {
                log::debug!("watching file {}", path.display());
            }
        }
        for res in rx {
            match res {
                Ok(events) => {
                    for event in events {
                        use notify::EventKind::*;
                        use notify::event::{AccessKind::*, AccessMode::*, ModifyKind::*};

                        let (Access(Close(Write)) | Modify(Data(_))) = event.kind else { continue };
                        for shader in event.paths.iter()
                            .filter_map(|path| shaders_by_path.get(path))
                        {
                            let Some(path) = &shader.path else { continue };
                            log::info!("shader changed {}", path.display());
                            let Ok(mut inner) = shader.inner.write() else {
                                log::error!("Lock poisoned");
                                continue;
                            };
                            inner.code_has_changed = true;
                        }
                    }
                }
                Err(e) => log::info!("watch error: {:?}", e),
            }
        }
    });
}

pub struct HotShader {
    path: Option<PathBuf>,
    shader_kind: ShaderKind,
    inner: RwLock<HotShaderInner>,
}

impl HotShader {
    pub fn new<P: Into<PathBuf>>(path: P, shader_kind: ShaderKind) -> Self {
        Self {
            path: Some(path.into()),
            shader_kind,
            inner: RwLock::new(HotShaderInner {
                code_has_changed: true,
                ..Default::default()
            }),
        }
    }

    pub fn new_nonhot(module: Arc<ShaderModule>, shader_kind: ShaderKind) -> Self {
        Self {
            path: None,
            shader_kind,
            inner: RwLock::new(HotShaderInner {
                module: Some(module),
                ..Default::default()
            }),
        }
    }

    pub fn new_vert<P: Into<PathBuf>>(path: P) -> Self {
        Self::new(path, ShaderKind::Vertex)
    }

    pub fn new_frag<P: Into<PathBuf>>(path: P) -> Self {
        Self::new(path, ShaderKind::Fragment)
    }

    pub fn set_device(&self, device: Arc<Device>) {
        let mut inner = self.inner.write().unwrap();
        inner.device = Some(device);
    }

    pub fn get_module(&self) -> anyhow::Result<Option<Arc<ShaderModule>>> {
        let inner = self.inner.read().map_err(|_| anyhow::anyhow!("Lock poisoned"))?;
        Ok(inner.module.clone())
    }

    /// Reloads shader if changed or `forced` is `true`.
    /// Returns `true` if shader is recompiling.
    pub fn reload(self: &Arc<Self>, forced: bool) -> bool {
        let path = self.path.clone().expect("shader must have a path set to load it");
        let mut inner = self.inner.write().unwrap();
        if inner.is_compiling {
            return true;
        }
        if !inner.code_has_changed && !forced {
            return false;
        }

        // reset code_has_changed here so we don't loop if an error happens
        inner.code_has_changed = false;
        inner.module = None;

        let sender = COMPILE_THREAD.clone();
        match sender.send(self.clone()) {
            Ok(_) => {
                inner.is_compiling = true;
                log::debug!("queued shader for recompilation {}", path.display());
                true
            }
            Err(err) => {
                log::error!("failed to queue shader for recompilation: {err}");
                false
            }
        }
    }

    fn compile_code(&self) -> anyhow::Result<()> {
        let inner = self.inner.read().map_err(|_| anyhow::anyhow!("Lock poisoned"))?;
        let Some(device) = inner.device.clone() else {
            return Err(anyhow::anyhow!("device not set"));
        };
        drop(inner);
        // Compiling takes some time, do not keep a lock while compiling!
        let result = self.compile_code_helper(device);
        let mut inner = self.inner.write().map_err(|_| anyhow::anyhow!("Lock poisoned"))?;
        inner.is_compiling = false;
        match result {
            Ok(module) => {
                inner.module = Some(module);
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    fn compile_code_helper(&self, device: Arc<Device>) -> anyhow::Result<Arc<ShaderModule>> {
        let Some(path) = self.path.as_ref() else {
            return Err(anyhow::anyhow!("cannot compile non hot shader"));
        };
        let module = HotShaderInner::compile(path, self.shader_kind, device)?;
        Ok(module)
    }
}

impl Default for HotShader {
    fn default() -> Self {
        Self {
            path: Default::default(),
            // this is just some arbitrary value that should never be used
            shader_kind: ShaderKind::DefaultVertex,
            inner: Default::default(),
        }
    }
}

#[derive(Default)]
pub struct HotShaderInner {
    device: Option<Arc<Device>>,
    is_compiling: bool,
    code_has_changed: bool,
    module: Option<Arc<ShaderModule>>,
}

impl HotShaderInner {
    fn compile(path: &Path, kind: ShaderKind, device: Arc<Device>)
        -> anyhow::Result<Arc<ShaderModule>>
    {
        log::debug!("compiling shader {} of kind {:?}", path.display(), kind);
        let start = Instant::now();
        let source = fs::read_to_string(path)?;
        let compiler = Compiler::new()
            .ok_or_else(|| anyhow::anyhow!("failed to get compiler"))?;
        let mut options = CompileOptions::new()
            .ok_or_else(|| anyhow::anyhow!("failed to get compile options"))?;
        options.set_include_callback(|name, _ty, src, depth| {
            // ty returns always IncludeType::Standard for some reason
            // just ignore it and assume IncludeType::Relative
            /*
            if let IncludeType::Standard = ty {
                return Err(r#"Standard includes (#include <...>) are not supported, please use relative includes (#include "...")."#.to_owned());
            }
            */

            if depth > MAX_INCLUDE_DEPTH {
                return Err(format!("Exceeded max include depth of {MAX_INCLUDE_DEPTH}."));
            }

            let path = Path::new(src);
            let path = path.parent().unwrap_or(path).join(name);
            let content = match std::fs::read_to_string(&path) {
                Ok(content) => content,
                Err(err) => {
                    return Err(format!("Failed to read file {}: {err}", path.display()));
                }
            };
            Ok(ResolvedInclude {
                resolved_name: path.to_string_lossy().into_owned(),
                content,
            })
        });

        let binary_result = compiler.compile_into_spirv(
            &source,
            kind,
            &path.to_string_lossy(),
            "main",
            Some(&options)
        )?;
        let code = binary_result.as_binary();
        let module = unsafe {
            ShaderModule::new(device, ShaderModuleCreateInfo::new(code))?
        };
        let time = start.elapsed();
        log::debug!("done compiling, took {time:?}");
        Ok(module)
    }
}
