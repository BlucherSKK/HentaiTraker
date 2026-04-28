use rocket::http::ext::IntoCollection;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

// ----- Permission -----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum Permission {
    Read        = 0,
    Write       = 1,
    Delete      = 2,
    ManageUsers = 3,
    ManageRoles = 4,
    Ban         = 5,
    Posting     = 6,
    Terminal    = 7,
}

impl Permission {
    pub const ALL: &'static [Permission] = &[
        Permission::Read,
        Permission::Write,
        Permission::Delete,
        Permission::ManageUsers,
        Permission::ManageRoles,
        Permission::Ban,
        Permission::Posting,
        Permission::Terminal,
    ];

    pub fn as_i32(self) -> i32 { self as i32 }

    pub fn from_i32(i: i32) -> Option<Self> {
        match i {
            0 => Some(Self::Read),
            1 => Some(Self::Write),
            2 => Some(Self::Delete),
            3 => Some(Self::ManageUsers),
            4 => Some(Self::ManageRoles),
            5 => Some(Self::Ban),
            6 => Some(Self::Posting),
            7 => Some(Self::Terminal),
            _ => None,
        }
    }

    pub fn force_role_name(self) -> &'static str {
        match self {
            Self::Read        => "force_read",
            Self::Write       => "force_write",
            Self::Delete      => "force_delete",
            Self::ManageUsers => "force_manage_users",
            Self::ManageRoles => "force_manage_roles",
            Self::Ban         => "force_ban",
            Self::Posting     => "force_posting",
            Self::Terminal    => "force_terminal",
        }
    }
}

// ----- Role / UserRole -----

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Role {
    pub id:          i32,
    pub name:        String,
    pub permissions: Vec<i32>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserRole {
    pub user_id: i32,
    pub role_id: i32,
}
// ----- helpers -----

pub fn resolve_permissions(roles: &[Role]) -> Vec<i32> {
    let mut perms: Vec<i32> = roles
    .iter()
    .flat_map(|r| r.permissions.iter().copied())
    .collect();
    perms.sort_unstable();
    perms.dedup();
    if perms.contains(&-1) {
        return Permission::ALL.iter().map(|p| p.as_i32()).collect();
    }
    perms
}

pub fn role_names(roles: &[Role]) -> Vec<String> {
    roles.iter().map(|r| r.name.clone()).collect()
}

// ----- bootstrap -----

pub async fn init_roles(pool: &PgPool) -> Result<(), sqlx::Error> {
    let all_perms: Vec<i32> = Permission::ALL.iter().map(|p| p.as_i32()).collect();

    sqlx::query(
        "INSERT INTO roles (name, permissions) VALUES ($1, $2) ON CONFLICT (name) DO NOTHING"
    )
    .bind("admin")
    .bind(&all_perms)
    .execute(pool).await?;

    for perm in Permission::ALL {
        sqlx::query(
            "INSERT INTO roles (name, permissions) VALUES ($1, $2) ON CONFLICT (name) DO NOTHING"
        )
        .bind(perm.force_role_name())
        .bind(vec![perm.as_i32()])
        .execute(pool).await?;
    }

    sqlx::query(r#"
    INSERT INTO user_roles (user_id, role_id)
    SELECT 1, id FROM roles WHERE name = 'admin'
    ON CONFLICT DO NOTHING
    "#)
    .execute(pool).await?;

    Ok(())
}
