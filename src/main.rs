use axum::{response::Html, routing::get, Router};
use tower_http::services::{ServeDir, ServeFile};

#[tokio::main]
async fn main() {
    // 配置当访问不存在 url 时的默认返回
    let serve_dir =
        ServeDir::new("assets2").not_found_service(ServeFile::new("assets2/index.html")); // not_found_service 传入的是默认获取的文件

    // 使用路由构建应用程序
    let app = Router::new()
        .route("/", get(handler))
        .nest_service("/assets", ServeDir::new("assets")) // 把 /assets/* 的 URL 映射到 assets 目录下
        .nest_service("/assets2", serve_dir.clone())
        .fallback_service(serve_dir); // 注意需要挂载

    // 启动端口监听
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    
    axum::serve(listener, app).await.unwrap();
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}
