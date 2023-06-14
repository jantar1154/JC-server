use actix_web::{App, HttpResponse, HttpServer, Responder, post, web};

#[derive(serde::Deserialize)]
struct Args {
    author: String,
    date: i64,
    message: String
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Message {
    message: String,
}

// this funciton handles JSON input and appends it to the SQLite database
#[post("/addpost")]
async fn add_post(args: web::Json<Args>) -> impl Responder {
    let conn = sqlite::open("database.db").unwrap();

    let author: &str = &args.author;
    let date: &str = &args.date.to_string();
    let msg: &str = &args.message;

    let sql = "INSERT INTO posts (author, date, message) 
    VALUES (
        :author,
        :date,
        :msg
    )";

    let mut stmt = conn.prepare(sql).unwrap();
    stmt.bind((":author", author)).unwrap();
    stmt.bind((":date", date)).unwrap();
    stmt.bind((":msg", msg)).unwrap();

    while let Ok(sqlite::State::Row) = stmt.next(){};
    
    drop(&stmt);
    drop(&conn);

    return HttpResponse::Ok().json(Message{
        message: format!("{} ({}) says: {}", author, date, msg)
    });
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let conn: sqlite::Connection = sqlite::open("database.db").expect("Could not connect to database!");
    init_db(&conn);

    // DEBUG: prints out all the database contents
    let sql: &str = "SELECT * FROM posts";
    let mut stmt = conn.prepare(sql).unwrap();

    while let Ok(sqlite::State::Row) = stmt.next() {
        // reads all rows from db and displays them
        let id: i64 = stmt.read::<i64, _>("ID_post").unwrap();
        let athr: String = stmt.read::<String, _>("author").unwrap();
        let date: i64 = stmt.read::<i64, _>("date").unwrap();
        let msg: String = stmt.read::<String, _>("message").unwrap();

        // prints out read values
        println!("({}). Name {}, date ({}), msg: {}", id, athr, date, msg);
    }

    drop(&conn);

    return HttpServer::new(|| App::new()
    .service(add_post))
        .bind("0.0.0.0:6950")?
        .run()
        .await;
}

fn init_db(conn: &sqlite::Connection) {
    let sql: &str = "
        CREATE TABLE IF NOT EXISTS posts (
            ID_post INTEGER NOT NULL PRIMARY KEY, 
            author VARCHAR, 
            date INTEGER, 
            message VARCHAR
        );
    ";

    conn.execute(sql).expect("Unable to create database!"); //creates the database
}
