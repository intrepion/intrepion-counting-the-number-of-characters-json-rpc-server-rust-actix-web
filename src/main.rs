use actix_web::{middleware, web, App, HttpResponse, HttpServer};
use serde::{Deserialize, Serialize};
use std::{env, net::TcpListener};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Deserialize, Serialize)]
struct CharCountParams {
    some_string: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct CharCountRequest {
    id: String,
    jsonrpc: String,
    method: String,
    params: CharCountParams,
}

#[derive(Debug, Serialize)]
struct CharCountResponse {
    id: String,
    jsonrpc: String,
    result: CharCountResult,
}

#[derive(Debug, Serialize)]
struct CharCountResult {
    count: i32,
}

#[derive(Debug, Serialize)]
struct MethodNotFoundError {
    code: i32,
    message: String,
}

#[derive(Debug, Serialize)]
struct MethodNotFoundErrorResponse {
    error: MethodNotFoundError,
    id: String,
    jsonrpc: String,
}

pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

async fn json_rpc_handler(item: web::Json<CharCountRequest>) -> HttpResponse {
    match item.method.as_str() {
        "char_count" => {
            let some_string = item.params.some_string.trim();
            let count = some_string.graphemes(true).count() as i32;
            let response = CharCountResponse {
                id: item.id.clone(),
                jsonrpc: item.jsonrpc.clone(),
                result: CharCountResult { count },
            };

            HttpResponse::Ok().json(response)
        }
        _ => {
            let response = MethodNotFoundErrorResponse {
                error: MethodNotFoundError {
                    code: -32601,
                    message: "Method not found".to_string(),
                },
                id: item.id.clone(),
                jsonrpc: item.jsonrpc.clone(),
            };

            HttpResponse::Ok().json(response)
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    log::info!("starting HTTP server at http://localhost:8080");

    let app_url = env::var("APP_URL").unwrap_or_else(|_| "127.0.0.1".to_string());

    let address = format!("{app_url}:8000");
    let listener = TcpListener::bind(&address)?;

    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default())
            .app_data(web::JsonConfig::default().limit(4096))
            .route("/", web::post().to(json_rpc_handler))
            .route("/health_check", web::get().to(health_check))
    })
    .listen(listener)?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use crate::{
        json_rpc_handler, CharCountParams, CharCountRequest, CharCountResponse, CharCountResult,
    };
    use actix_web::{body::to_bytes, dev::Service, http, test, web, App};

    #[actix_web::test]
    async fn test_happy_paths() {
        let app = test::init_service(
            App::new().service(web::resource("/").route(web::post().to(json_rpc_handler))),
        )
        .await;

        let key_values = vec![("", 0), ("Oliver", 6)];

        for key_value in key_values {
            let req = test::TestRequest::post()
                .uri("/")
                .set_json(CharCountRequest {
                    id: "00000000-0000-0000-0000-000000000000".to_owned(),
                    jsonrpc: "2.0".to_owned(),
                    method: "char_count".to_owned(),
                    params: CharCountParams {
                        some_string: key_value.0.to_owned(),
                    },
                })
                .to_request();
            let resp = app.call(req).await.unwrap();

            assert_eq!(resp.status(), http::StatusCode::OK);

            let result = CharCountResponse {
                id: "00000000-0000-0000-0000-000000000000".to_owned(),
                jsonrpc: "2.0".to_owned(),
                result: CharCountResult { count: key_value.1 },
            };

            let actual = to_bytes(resp.into_body()).await.unwrap();
            let expected = serde_json::to_string(&result).unwrap();

            assert_eq!(actual, expected);
        }
    }

    #[actix_web::test]
    async fn test_other_possibilities() {
        let app = test::init_service(
            App::new().service(web::resource("/").route(web::post().to(json_rpc_handler))),
        )
        .await;

        let key_values = vec![
            (" ", 0),
            ("Oliver ", 6),
            (" Oliver", 6),
            (" Oliver ", 6),
            ("Olivér", 6),
        ];

        for key_value in key_values {
            let req = test::TestRequest::post()
                .uri("/")
                .set_json(CharCountRequest {
                    id: "00000000-0000-0000-0000-000000000000".to_owned(),
                    jsonrpc: "2.0".to_owned(),
                    method: "char_count".to_owned(),
                    params: CharCountParams {
                        some_string: key_value.0.to_owned(),
                    },
                })
                .to_request();
            let resp = app.call(req).await.unwrap();

            assert_eq!(resp.status(), http::StatusCode::OK);

            let result = CharCountResponse {
                id: "00000000-0000-0000-0000-000000000000".to_owned(),
                jsonrpc: "2.0".to_owned(),
                result: CharCountResult {
                    count: key_value.1.to_owned(),
                },
            };

            let actual = to_bytes(resp.into_body()).await.unwrap();
            let expected = serde_json::to_string(&result).unwrap();

            assert_eq!(actual, expected);
        }
    }

    #[actix_web::test]
    async fn test_non_existant_method() {
        let app = test::init_service(
            App::new().service(web::resource("/").route(web::post().to(json_rpc_handler))),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/")
            .set_json(&CharCountRequest {
                id: "00000000-0000-0000-0000-000000000000".to_owned(),
                jsonrpc: "2.0".to_owned(),
                method: "wrong".to_owned(),
                params: CharCountParams {
                    some_string: "Oliver".to_owned(),
                },
            })
            .to_request();
        let resp = app.call(req).await.unwrap();

        assert_eq!(resp.status(), http::StatusCode::OK);

        let body_bytes = to_bytes(resp.into_body()).await.unwrap();
        assert_eq!(
            body_bytes,
            r##"{"error":{"code":-32601,"message":"Method not found"},"id":"00000000-0000-0000-0000-000000000000","jsonrpc":"2.0"}"##
        );
    }
}
