use std::{hash::Hasher};

use actix_web::{App, HttpResponse, HttpServer, Responder, post, web, dev::Response};
use rs_sha384::{Sha384Hasher, HasherContext};

#[derive(serde::Deserialize)]
struct ArgsPost {
    uname: String,
    pass: String,
    token: String,
    date: i64,
    title: String,
    message: String
}

#[derive(serde::Deserialize)]
struct ArgsNewAcc {
    uname: String,
    pass: String
}

#[derive(serde::Deserialize)]
struct ArgsGetToken {
    uname: String,
    pass: String
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Message {
    message: String,
}

// this function handles creating an user account
#[post("/newacc")]
async fn new_acc(args: web::Json<ArgsNewAcc>) -> impl Responder {
    let mut response = "success";
    let mut create_acc = true; // Will be set to false if entry should not be done
    let conn = conn_users();

    let uname: &str = &args.uname;
    let pass: &str = &pass_hasher(&args.pass);

    // Check whether uname is already located in ./users.db
    // If so dont create that account
    let sql = "SELECT * FROM users WHERE uname=:uname";
    let mut stmt= conn.prepare(sql).unwrap();
    stmt.bind((":uname", uname)).unwrap();
    while let Ok(sqlite::State::Row) = stmt.next() {
        let found_name = stmt.read::<String, _>("uname").unwrap();
        println!("New acc: {}, found name: {}", uname, found_name);
        response = "Username already taken";
        create_acc = false;
    };

    let token: &str = &generate_token(uname, pass);

    let sql = "
        INSERT INTO users (uname, pass, token) 
        VALUES (
            :uname, 
            :pass,
            :token
        );
    ";

    let mut stmt = conn.prepare(sql).unwrap();
    stmt.bind((":uname", uname)).unwrap();
    stmt.bind((":pass", pass)).unwrap();
    stmt.bind((":token", token)).unwrap();

    if create_acc {
        // If all is good then create the entry
        while let Ok(sqlite::State::Row) = stmt.next(){};
    }

    return HttpResponse::Ok().json(Message{
        message: response.to_string()
    });
}

#[post("/gettkn")]
async fn get_token(args: web::Json<ArgsGetToken>) -> impl Responder {
    let uname = &args.uname;
    let pass = &args.pass;
    let token = generate_token(uname, pass);

    let mut response = "invalid credentials";

    if check_login(uname, pass, &token) {
        response = &token;
    }

    return HttpResponse::Ok().json(Message{
        message: response.to_string()
    });
}

// this funciton handles JSON input and appends it to the SQLite database
#[post("/addpost")]
async fn add_post(args: web::Json<ArgsPost>) -> impl Responder {
    let mut response = "success";
    let conn = conn_posts();

    let uname: &str = &args.uname;
    let pass: &str = &args.pass;
    let date: &str = &args.date.to_string();
    let title: &str = &args.title;
    let msg: &str = &args.message;
    let token: &str = &args.token;

    let sql = "INSERT INTO posts (author, date, title, message) 
    VALUES (
        :uname,
        :date,
        :title,
        :msg
    )";

    match check_login(uname, pass, token) {
        true => {
            let mut stmt = conn.prepare(sql).unwrap();
            stmt.bind((":uname", uname)).unwrap();
            stmt.bind((":date", date)).unwrap();
            stmt.bind((":title", title)).unwrap();
            stmt.bind((":msg", msg)).unwrap();
            while let Ok(sqlite::State::Row) = stmt.next(){};
            drop(&stmt);
        }
        false => response = "invalid credentials"
    }
    

    return HttpResponse::Ok().json(Message{
        message: response.to_string()
    });
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let conn_posts: sqlite::Connection = conn_posts();
    let conn_users: sqlite::Connection = conn_users();

    init_post(&conn_posts);
    init_users(&conn_users);
    
    let sql: &str = "SELECT * FROM posts";
    let mut stmt = conn_posts.prepare(sql).unwrap();

    while let Ok(sqlite::State::Row) = stmt.next() {
        // reads all rows from db and displays them
        let id: i64 = stmt.read::<i64, _>("ID_post").unwrap();
        let athr: String = stmt.read::<String, _>("author").unwrap();
        let date: i64 = stmt.read::<i64, _>("date").unwrap();
        let msg: String = stmt.read::<String, _>("message").unwrap();

        // prints out read values
        println!("({}). Name {}, date ({}), msg: {}", id, athr, date, msg);
    }

    return HttpServer::new(|| App::new()
    .service(add_post)
    .service(new_acc)
    .service(get_token))
        .bind("0.0.0.0:6950")?
        .run()
        .await;
}

fn conn_posts() -> sqlite::Connection {sqlite::open("posts.db").expect("Could not connect to database!")}

fn conn_users() -> sqlite::Connection {sqlite::open("users.db").expect("Could not connect to database!")}

fn init_post(conn: &sqlite::Connection) {
    // creates the posts database (if it doesn't exist)
    let sql: &str = "
        CREATE TABLE IF NOT EXISTS posts (
            ID_post INTEGER NOT NULL PRIMARY KEY,
            author VARCHAR,
            date INTEGER,
            title VARCHAR,
            message VARCHAR
        );
    ";

    conn.execute(sql).unwrap();
}

fn init_users(conn: &sqlite::Connection) {
    //creates the users database
    let sql: &str = "
        CREATE TABLE IF NOT EXISTS users (
            ID_usr INTEGER NOT NULL PRIMARY KEY, 
            uname VARCHAR, 
            pass VARCHAR,
            token VARCHAR
        );
    ";

    conn.execute(sql).unwrap();
}

fn pass_hasher(str: &str) -> String {
    let mut hasher: Sha384Hasher = Sha384Hasher::default();
    hasher.write(str.as_bytes());
    let result = HasherContext::finish(&mut hasher);
    return format!("{result:02x}");
}

fn check_login(uname: &str, pass: &str, token: &str) -> bool {
    let conn = conn_users();
    let sql = "
        SELECT * FROM users WHERE uname=:uname
    ";
    let mut stmt = conn.prepare(sql).unwrap();
    stmt.bind((":uname", uname)).unwrap();
    let pass_hashed = pass_hasher(pass);

    while let Ok(sqlite::State::Row) = stmt.next(){
        let pass_db = stmt.read::<String, _>("pass").unwrap();
        if pass_db == pass_hashed && token == generate_token(&uname, &pass){
            return true;
        }
    };
    return false;
}

fn generate_token(uname: &str, pass: &str) -> String {
    let res = pass_hasher(&format!("696969{}1as23dfgh1456{}aujisdhfgbasdbnhujisbg{}f45d6ah4156{}", uname, pass, uname, uname));
    return res;
}

fn is_token_in_db(token: &str) -> bool {
    let conn = conn_users();

    let sql = "
        SELECT * FROM users WHERE token=:token
    ";
    let mut stmt = conn.prepare(sql).unwrap();
    stmt.bind((":token", token)).unwrap();

    while let Ok(sqlite::State::Row) = stmt.next() {
        return true;
    }
    return false;
}