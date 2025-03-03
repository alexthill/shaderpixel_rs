use shaderc::{Compiler, CompileOptions, ShaderKind};
use vulkano::{
    device::Device,
    shader::{ShaderModule, ShaderModuleCreateInfo},
};

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, LazyLock, RwLock};
use std::thread;
use std::time::Instant;

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
        let compiler = Compiler::new().unwrap();
        let options = CompileOptions::new().unwrap();
        let binary_result = compiler.compile_into_spirv(
            &source,
            kind,
            path.to_str().unwrap_or("shader.glsl"),
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
