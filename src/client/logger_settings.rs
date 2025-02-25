use logger::LogLevel;

use super::ClientVideo;

impl ClientVideo {
    /// Disable all logs
    /// # Panics
    /// May panic if the `RwLock` is poisone
    pub(crate) fn with_info(&self) {
        self.state
            .write()
            .logger
            .set_displayable(LogLevel::Info as u8);
    }

    /// Enable debug logs
    /// # Panics
    /// May panic if the `RwLock` is poisoned
    pub(crate) fn with_debug(&self) {
        self.state
            .write()
            .logger
            .set_displayable(LogLevel::Debug as u8);
    }

    /// Enable error logs
    /// # Panics
    /// May panic if the `RwLock` is poisoned
    pub(crate) fn with_error(&self) {
        self.state
            .write()
            .logger
            .set_displayable(LogLevel::Error as u8);
    }

    /// Enable warning logs
    /// # Panics
    /// May panic if the `RwLock` is poisoned
    pub(crate) fn with_warning(&self) {
        self.state
            .write()
            .logger
            .set_displayable(LogLevel::Warn as u8);
    }

    /// Enable all logs
    /// # Panics
    /// May panic if the `RwLock` is poisoned
    pub(crate) fn with_all(&self) {
        self.state
            .write()
            .logger
            .set_displayable(LogLevel::All as u8);
    }

    /// Enable logs to be displayed in the console
    /// # Panics
    /// May panic if the `RwLock` is poisoned
    pub(crate) fn with_web_socket(&self) {
        self.state.write().logger.init_web_socket();
    }
}
