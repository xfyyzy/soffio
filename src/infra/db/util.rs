use crate::application::repos::RepoError;

pub fn map_sqlx_error(err: sqlx::Error) -> RepoError {
    match err {
        sqlx::Error::RowNotFound => RepoError::NotFound,
        sqlx::Error::Database(db) if db.message().contains("duplicate key") => {
            RepoError::Duplicate {
                constraint: db.constraint().unwrap_or("unknown").to_string(),
            }
        }
        sqlx::Error::Database(db)
            if db.message().contains("violates foreign key constraint")
                || db.message().contains("invalid input syntax") =>
        {
            RepoError::InvalidInput {
                message: db.message().to_string(),
            }
        }
        sqlx::Error::Database(db) if db.message().contains("violates") => RepoError::Integrity {
            message: db.message().to_string(),
        },
        sqlx::Error::Database(db)
            if db
                .message()
                .contains("canceling statement due to user request") =>
        {
            RepoError::Timeout
        }
        other => RepoError::from_persistence(other),
    }
}
