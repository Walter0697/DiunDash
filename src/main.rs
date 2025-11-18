use actix_web::{web, App, HttpServer, Responder, HttpResponse, HttpRequest};
use actix_files::Files;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Mutex;
use rusqlite::{Connection, params};
use chrono::Utc;
use std::fs;
use std::env;

#[derive(Debug, Deserialize, Serialize)]
struct DiunWebhook {
    created: String,
    digest: String,
    diun_version: String,
    hostname: String,
    hub_link: String,
    image: String,
    metadata: Option<serde_json::Value>,
    mime_type: String,
    platform: String,
    provider: String,
    status: String,
}

#[derive(Debug, Serialize)]
struct ImageRecord {
    image: String,
    created: String,
    digest: String,
    diun_version: String,
    hostname: String,
    hub_link: String,
    metadata: Option<String>,
    mime_type: String,
    platform: String,
    provider: String,
    status: String,
    updated_at: String,
}

async fn hello() -> impl Responder {
    HttpResponse::Ok().body(r#"
        <h2>Hello, World! 🌍</h2>
        <p>This response was loaded via HTMX!</p>
    "#)
}

async fn verify_api_key(
    body: web::Json<serde_json::Value>,
    api_key: web::Data<String>,
) -> impl Responder {
    let provided_key = match body.get("api_key") {
        Some(key) => match key.as_str() {
            Some(k) => k,
            None => {
                return HttpResponse::BadRequest().json(serde_json::json!({
                    "status": "error",
                    "message": "Invalid API key format"
                }));
            }
        },
        None => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "status": "error",
                "message": "Missing API key"
            }));
        }
    };
    
    if provided_key == api_key.as_str() {
        HttpResponse::Ok().json(serde_json::json!({
            "status": "success",
            "message": "API key verified"
        }))
    } else {
        HttpResponse::Unauthorized().json(serde_json::json!({
            "status": "error",
            "message": "Invalid API key"
        }))
    }
}

async fn delete_image_handler_with_auth(
    req: HttpRequest,
    path: web::Path<String>,
    db: web::Data<Mutex<Connection>>,
    api_key: web::Data<String>,
) -> impl Responder {
    // Check Authorization header
    let auth_result = req.headers().get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| {
            if s.starts_with("Bearer ") {
                Some(&s[7..])
            } else {
                None
            }
        });
    
    // Validate authorization
    match auth_result {
        Some(token) if token == api_key.as_str() => {
            // Auth passed, call the original handler
            delete_image_handler(path, db).await
        }
        _ => {
            HttpResponse::Unauthorized().json(serde_json::json!({
                "status": "error",
                "message": "Invalid or missing API key"
            }))
        }
    }
}

async fn admin() -> impl Responder {
    match fs::read_to_string("./static/index.html") {
        Ok(content) => HttpResponse::Ok()
            .content_type("text/html")
            .body(content),
        Err(_) => HttpResponse::NotFound().body("Admin page not found"),
    }
}

fn init_database() -> Result<Connection, rusqlite::Error> {
    std::fs::create_dir_all("./data").expect("Failed to create data directory");
    let conn = Connection::open("./data/diun.db")?;
    
    conn.execute(
        "CREATE TABLE IF NOT EXISTS webhooks (
            image TEXT PRIMARY KEY,
            created TEXT NOT NULL,
            digest TEXT NOT NULL,
            diun_version TEXT NOT NULL,
            hostname TEXT NOT NULL,
            hub_link TEXT NOT NULL,
            metadata TEXT,
            mime_type TEXT NOT NULL,
            platform TEXT NOT NULL,
            provider TEXT NOT NULL,
            status TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        [],
    )?;
    
    Ok(conn)
}

fn upsert_webhook(conn: &Mutex<Connection>, webhook: &DiunWebhook) -> Result<(), rusqlite::Error> {
    let conn = conn.lock().unwrap();
    let updated_at = Utc::now().to_rfc3339();
    let metadata_str = webhook.metadata.as_ref()
        .and_then(|m| serde_json::to_string(m).ok());
    
    conn.execute(
        "INSERT INTO webhooks (
            image, created, digest, diun_version, hostname, hub_link,
            metadata, mime_type, platform, provider, status, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
        ON CONFLICT(image) DO UPDATE SET
            created = ?2,
            digest = ?3,
            diun_version = ?4,
            hostname = ?5,
            hub_link = ?6,
            metadata = ?7,
            mime_type = ?8,
            platform = ?9,
            provider = ?10,
            status = ?11,
            updated_at = ?12",
        params![
            webhook.image,
            webhook.created,
            webhook.digest,
            webhook.diun_version,
            webhook.hostname,
            webhook.hub_link,
            metadata_str,
            webhook.mime_type,
            webhook.platform,
            webhook.provider,
            webhook.status,
            updated_at
        ],
    )?;
    
    Ok(())
}

fn get_all_images(conn: &Mutex<Connection>) -> Result<Vec<ImageRecord>, rusqlite::Error> {
    let conn = conn.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT image, created, digest, diun_version, hostname, hub_link,
         metadata, mime_type, platform, provider, status, updated_at
         FROM webhooks ORDER BY updated_at DESC"
    )?;
    
    let image_iter = stmt.query_map([], |row| {
        Ok(ImageRecord {
            image: row.get(0)?,
            created: row.get(1)?,
            digest: row.get(2)?,
            diun_version: row.get(3)?,
            hostname: row.get(4)?,
            hub_link: row.get(5)?,
            metadata: row.get(6)?,
            mime_type: row.get(7)?,
            platform: row.get(8)?,
            provider: row.get(9)?,
            status: row.get(10)?,
            updated_at: row.get(11)?,
        })
    })?;
    
    let mut images = Vec::new();
    for image in image_iter {
        images.push(image?);
    }
    
    Ok(images)
}

fn delete_image(conn: &Mutex<Connection>, image: &str) -> Result<(), rusqlite::Error> {
    let conn = conn.lock().unwrap();
    conn.execute("DELETE FROM webhooks WHERE image = ?1", params![image])?;
    Ok(())
}

async fn delete_image_handler(
    path: web::Path<String>,
    db: web::Data<Mutex<Connection>>,
) -> HttpResponse {
    let image = path.into_inner();
    
    match delete_image(&db, &image) {
        Ok(_) => {
            HttpResponse::Ok().json(serde_json::json!({
                "status": "success",
                "message": "Image deleted successfully"
            }))
        }
        Err(e) => {
            eprintln!("✗ Database error: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "status": "error",
                "message": "Failed to delete image from database"
            }))
        }
    }
}

async fn list_images(
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    match get_all_images(&db) {
        Ok(images) => {
            HttpResponse::Ok().json(serde_json::json!({
                "status": "success",
                "count": images.len(),
                "images": images
            }))
        }
        Err(e) => {
            eprintln!("✗ Database error: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "status": "error",
                "message": "Failed to retrieve images from database"
            }))
        }
    }
}

async fn diun_webhook(
    body: web::Json<DiunWebhook>,
    db: web::Data<Mutex<Connection>>,
) -> HttpResponse {
    let webhook = body.into_inner();
    
    println!("=== Diun Webhook Received ===");
    println!("Image: {}", webhook.image);
    println!("Status: {}", webhook.status);
    println!("Digest: {}", webhook.digest);
    println!("Platform: {}", webhook.platform);
    println!("Provider: {}", webhook.provider);
    println!("Hostname: {}", webhook.hostname);
    println!("Created: {}", webhook.created);
    println!("Diun Version: {}", webhook.diun_version);
    println!("MIME Type: {}", webhook.mime_type);
    if let Some(metadata) = &webhook.metadata {
        println!("Metadata: {}", serde_json::to_string_pretty(metadata).unwrap());
    }
    
    // Upsert to database
    match upsert_webhook(&db, &webhook) {
        Ok(_) => {
            println!("✓ Successfully saved to database");
            HttpResponse::Ok().json(serde_json::json!({
                "status": "received",
                "message": "Webhook processed and saved successfully"
            }))
        }
        Err(e) => {
            eprintln!("✗ Database error: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "status": "error",
                "message": "Failed to save webhook to database"
            }))
        }
    }
}

async fn diun_webhook_with_auth(
    req: HttpRequest,
    body: web::Json<DiunWebhook>,
    db: web::Data<Mutex<Connection>>,
    api_key: web::Data<String>,
) -> impl Responder {
    // Check Authorization header
    let auth_result = req.headers().get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| {
            if s.starts_with("Bearer ") {
                Some(&s[7..])
            } else {
                None
            }
        });
    
    // Validate authorization
    match auth_result {
        Some(token) if token == api_key.as_str() => {
            // Auth passed, call the original handler
            diun_webhook(body, db).await
        }
        _ => {
            HttpResponse::Unauthorized().json(serde_json::json!({
                "status": "error",
                "message": "Invalid or missing API key"
            }))
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load .env file if it exists (optional, won't fail if missing)
    dotenv::dotenv().ok();
    
    // Get API key from environment variable
    let api_key = env::var("DIUNDASH_API_KEY")
        .expect("DIUNDASH_API_KEY environment variable must be set");
    
    // Initialize database
    let conn = init_database().expect("Failed to initialize database");
    
    let db = web::Data::new(Mutex::new(conn));
    let api_key_data = web::Data::new(api_key);
    
    HttpServer::new(move || {
        App::new()
            .app_data(db.clone())
            .app_data(api_key_data.clone())
            .route("/api/hello", web::get().to(hello))
            .route("/api/diun", web::post().to(diun_webhook_with_auth))
            .route("/api/verify", web::post().to(verify_api_key))
            .route("/api/images", web::get().to(list_images))
            .route("/api/images/{image}", web::delete().to(delete_image_handler_with_auth))
            .route("/admin", web::get().to(admin))
            .service(Files::new("/", "./static").index_file("index.html"))
    })
    .bind(SocketAddr::from(([0, 0, 0, 0], 5030)))?
    .run()
    .await
}

