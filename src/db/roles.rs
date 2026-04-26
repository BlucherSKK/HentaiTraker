use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

// ─── Permission enum ─────────────────────────────────────────────────────────

/// Хардкодированный список прав. Индексы (i32) хранятся в roles.permissions[].
/// Порядок — контракт: никогда не менять значения уже выпущенных вариантов.
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
}

impl Permission {
    /// Все варианты в порядке возрастания индекса.
    pub const ALL: &'static [Permission] = &[
        Permission::Read,
        Permission::Write,
        Permission::Delete,
        Permission::ManageUsers,
        Permission::ManageRoles,
        Permission::Ban,
        Permission::Posting,
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
            _ => None,
        }
    }

    /// Имя роли-одиночки для этого права, например "force_read".
    pub fn force_role_name(self) -> &'static str {
        match self {
            Self::Read        => "force_read",
            Self::Write       => "force_write",
            Self::Delete      => "force_delete",
            Self::ManageUsers => "force_manage_users",
            Self::ManageRoles => "force_manage_roles",
            Self::Ban         => "force_ban",
            Self::Posting     => "force_posting",
        }
    }
}

// ─── Domain structs ──────────────────────────────────────────────────────────

/// Запись из таблицы roles.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Role {
    pub id:          i32,
    pub name:        String,
    pub permissions: Vec<i32>,  // индексы Permission
}

/// Запись из связующей таблицы user_roles (many-to-many).
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserRole {
    pub user_id: i32,
    pub role_id: i32,
}

// ─── Bootstrap ───────────────────────────────────────────────────────────────

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

    // ── Bootstrap: user id=1 → admin ─────────────────────────────────────────
    sqlx::query(r#"
    INSERT INTO user_roles (user_id, role_id)
    SELECT 1, id FROM roles WHERE name = 'admin'
    ON CONFLICT DO NOTHING
    "#)
    .execute(pool).await?;

    Ok(())
}
