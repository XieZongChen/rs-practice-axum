use axum::{response::Html, routing::get, Router};
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // 配置当访问不存在 url 时的默认返回
    let serve_dir =
        ServeDir::new("assets2").not_found_service(ServeFile::new("assets2/index.html")); // not_found_service 传入的是默认获取的文件

    // 使用路由构建应用程序
    let app = Router::new()
        .route("/", get(handler))
        .nest_service("/assets", ServeDir::new("assets")) // 把 /assets/* 的 URL 映射到 assets 目录下
        .nest_service("/assets2", serve_dir.clone())
        .fallback_service(serve_dir) // 注意需要挂载
        .layer(TraceLayer::new_for_http()); // 日志中间件服务

    // 启动端口监听
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    /*
     * Rust 标准的 log 协议: https://docs.rs/log/latest/log/
     * 其中规定了 5 个级别的日志打印语句：error! | warn! | info! | debug! | trace!
     * 这 5 个级别从左到右警示程度为由高到低。而日志信息越往右会越详细。但是这只是一套协议的定义，而不是具体实现。
     * 具体使用的时候，需要用另外的 crate 来实现。我们常用的 env_logger 就是其中一种实现。
     * 而这里我们使用的 tracing 库也是这样一种实现。它是为 tokio 异步运行时专门设计的，适合在异步并发代码中使用。
     * 可以使用 RUST_LOG=trace cargo run 来启动项目并打开日志开关，日志会打印到终端。可以尝试将 trace 改为 debug，日志会少一些。
     */
    tracing::debug!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}
