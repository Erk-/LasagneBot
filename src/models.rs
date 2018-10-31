use chrono::NaiveDate;
use schema::comics;

#[derive(Insertable, Queryable, Debug)]
pub struct Comic {
    pub id: NaiveDate,
    pub fetch_count: i32,
}
