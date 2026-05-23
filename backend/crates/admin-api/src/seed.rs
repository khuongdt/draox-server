use crate::auth::AdminRole;
use crate::auth_store::{AdminUser, AdminUserStore};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use tracing::{error, info};

pub async fn seed_default_users(store: &AdminUserStore) {
    let defaults: &[(&str, &str, AdminRole)] = &[
        ("admin", "draox@Admin#2024", AdminRole::Admin),
        ("operator", "draox@Operator#2024", AdminRole::Operator),
        ("viewer", "draox@Viewer#2024", AdminRole::Viewer),
    ];

    for (username, password, role) in defaults {
        if store.exists(username).await {
            continue;
        }
        match hash_password(password) {
            Ok(hash) => {
                let user = AdminUser {
                    username: username.to_string(),
                    password_hash: hash,
                    role: *role,
                };
                if store.set(&user).await.is_ok() {
                    info!("seeded default user: {} ({:?})", username, role);
                }
            }
            Err(e) => {
                error!("failed to hash password for {}: {}", username, e);
            }
        }
    }
}

fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    Ok(argon2.hash_password(password.as_bytes(), &salt)?.to_string())
}
