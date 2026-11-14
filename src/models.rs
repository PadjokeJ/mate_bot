use diesel::prelude::*;

use crate::schema;

#[derive(Queryable, Selectable, Insertable, Clone)]
#[diesel(table_name = crate::schema::mates)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Mates {
    pub id: i64,
    pub display_name: String,
    pub count: i32,
}

impl Mates {
    pub fn total_count(conn: &mut SqliteConnection) -> i32 {
        Self::rows(conn, -1).iter().fold(0, |p, m| p + m.count)
    }
    pub fn rows(conn: &mut SqliteConnection, limit: i64) -> Vec<Mates> {
        schema::mates::dsl::mates
            .order_by(schema::mates::count.desc())
            .limit(limit)
            .select(Mates::as_select())
            .load(conn)
            .unwrap()
    }
}

#[derive(Queryable, Selectable, Insertable, Clone)]
#[diesel(table_name = crate::schema::allowed_chats)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct AllowedChats {
    pub id: i64,
}
