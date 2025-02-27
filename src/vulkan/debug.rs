use std::sync::Arc;

use vulkano::{
    instance::{
        debug::{
            DebugUtilsMessageSeverity, DebugUtilsMessageType, DebugUtilsMessenger,
            DebugUtilsMessengerCallback, DebugUtilsMessengerCreateInfo,
        },
        Instance, InstanceExtensions,
    },
    Validated, VulkanError, VulkanLibrary,
};

#[cfg(debug_assertions)]
const ENABLE_VALIDATION_LAYERS: bool = true;
#[cfg(not(debug_assertions))]
const ENABLE_VALIDATION_LAYERS: bool = false;

pub fn check_layer_support(library: &VulkanLibrary, layer_name: &str) -> Result<bool, VulkanError> {
    for layer in library.layer_properties()? {
        if layer.name() == layer_name {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn get_debug_extensions_and_layers() -> (InstanceExtensions, Vec<String>) {
    let extensions = InstanceExtensions {
        ext_debug_utils: ENABLE_VALIDATION_LAYERS,
        ..InstanceExtensions::empty()
    };

    let layers = if ENABLE_VALIDATION_LAYERS {
        vec!["VK_LAYER_KHRONOS_validation".to_owned()]
    } else {
        Vec::new()
    };

    (extensions, layers)
}

pub fn setup_debug_callback(
    instance: Arc::<Instance>,
) -> Result<Option<DebugUtilsMessenger>, Validated<VulkanError>> {
    if !ENABLE_VALIDATION_LAYERS {
        return Ok(None);
    }
    unsafe {
        let debug = DebugUtilsMessenger::new(
            instance,
            DebugUtilsMessengerCreateInfo {
                message_severity: DebugUtilsMessageSeverity::ERROR
                    | DebugUtilsMessageSeverity::WARNING
                    | DebugUtilsMessageSeverity::INFO
                    | DebugUtilsMessageSeverity::VERBOSE,
                message_type: DebugUtilsMessageType::GENERAL
                    | DebugUtilsMessageType::VALIDATION
                    | DebugUtilsMessageType::PERFORMANCE,
                ..DebugUtilsMessengerCreateInfo::user_callback(DebugUtilsMessengerCallback::new(
                    |message_severity, _message_type, callback_data| {
                        let message = &callback_data.message;
                        if message_severity
                            .intersects(DebugUtilsMessageSeverity::ERROR)
                        {
                            log::error!("{:?} {:?}", message_severity, message);
                        } else if message_severity.intersects(DebugUtilsMessageSeverity::WARNING) {
                            log::warn!("{:?} {:?}", message_severity, message);
                        } else if message_severity.intersects(DebugUtilsMessageSeverity::INFO) {
                            log::info!("{:?} {:?}", message_severity, message);
                        } else if message_severity.intersects(DebugUtilsMessageSeverity::VERBOSE) {
                            log::debug!("{:?} {:?}", message_severity, message);
                        } else {
                            log::trace!("{:?} {:?}", message_severity, message);
                        }
                    },
                ))
            },
        )?;
        Ok(Some(debug))
    }
}
