pub mod detector;
pub mod locale;
pub mod manager;
pub mod template;

pub use detector::{choose_locale, detect_from_header};
pub use locale::{Locale, DEFAULT_LOCALE};
pub use manager::{I18nError, I18nManager, MessageMap};
pub use template::render;
