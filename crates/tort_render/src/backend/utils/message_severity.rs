use ash::vk;
use log::{warn, Level};

#[inline]
pub fn to_log_level(message_severity: vk::DebugUtilsMessageSeverityFlagsEXT) -> Level {
    match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => Level::Trace,
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => Level::Info,
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => Level::Warn,
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => Level::Error,
        _ => {
            warn!(
                "Unknown {}: {}",
                "vk::DebugUtilsMessageSeverityFlagsEXT",
                message_severity.as_raw()
            );
            Level::Warn
        }
    }
}
