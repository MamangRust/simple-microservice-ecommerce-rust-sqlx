pub mod identity;
pub mod login;
pub mod password_reset;
pub mod register;
pub mod token;

pub use self::identity::{DynIdentityService, IdentityServiceTrait};
pub use self::login::{DynLoginService, LoginServiceTrait};
pub use self::password_reset::{DynPasswordResetService, PasswordServiceTrait};
pub use self::register::{DynRegisterService, RegisterServiceTrait};
pub use self::token::{DynTokenService, TokenServiceTrait};
