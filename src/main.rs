use axum::{
    extract::{rejection::JsonRejection, Form, Json, Query},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
    Router,
};
// serde 是 Rust 生态中用得最广泛的序列化和反序列化框架
use serde::Deserialize;
use serde_json::json;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

#[tokio::main]
async fn main() {
    /*
     * 这是一个 Collector，可以将记录的日志收集后，再输出到控制台中。
     * 收集的过程是通过通知的方式实现的：当 Event 发生或者 Span 开始/结束时，会调用 Collect 特征的相应方法通知 Collector。
     */
    tracing_subscriber::fmt::init();

    // 配置当访问不存在 url 时的默认返回
    let serve_dir =
        ServeDir::new("assets2").not_found_service(ServeFile::new("assets2/index.html")); // not_found_service 传入的是默认获取的文件

    // 使用路由构建应用程序
    let app = Router::new()
        .route("/", get(handler))
        .route("/query", get(query))
        .route("/form", get(show_form).post(accept_form))
        .route("/json", post(accept_json))
        .route("/handleParsingError", post(handle_parsing_error))
        .route("/handlerReturn", post(handler_return))
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

/**
 * 使用 Deserialize 属性后，Rust 编译器将自动生成实现 serde::Deserialize trait 的代码，
 * 这样就可以将数据（如 JSON，XML 等格式）反序列化为这个 struct
 */
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Params {
    foo: i32,
    bar: String,
    third: Option<i32>,
}

/**
 * GET 请求
 * params 参数就是我们想要的 query 请求参数，Axum 框架自动帮我们处理了解析工作，让我们直接得到了 Rust 结构体对象
 * Params 规定了这个请求接收的参数，以模式匹配的方式映射到 params 上
 * 对于可选参数，可以用 Option 声明。若请求有传入多余参数，多余的将会被忽略，params 只会取到 Params 中定义了的参数
 */
async fn query(Query(params): Query<Params>) -> Html<&'static str> {
    tracing::debug!("query params {:?}", params);
    Html("<h3>Test query</h3>")
}

async fn show_form() -> Html<&'static str> {
    Html(
        r#"
        <!doctype html>
        <html>
            <head></head>
            <body>
                <form action="/form" method="post">
                    <label for="name">
                        Enter your name:
                        <input type="text" name="name">
                    </label>

                    <label>
                        Enter your email:
                        <input type="text" name="email">
                    </label>

                    <input type="submit" value="Subscribe!">
                </form>
            </body>
        </html>
        "#,
    )
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct Input {
    name: String,
    email: String,
}

/**
 * POST Form 请求
 * 相比于前面的 query，form 代码结构完全一致，只是解包器由 Query 换成了 Form。这体现了 Axum 具有相当良好的人体工程学，使开发非常省力。
 */
async fn accept_form(Form(input): Form<Input>) -> Html<&'static str> {
    tracing::debug!("form params {:?}", input);
    Html("<h3>Form posted</h3>")
}

/**
 * POST Json 请求
 */
async fn accept_json(Json(input): Json<Input>) -> Html<&'static str> {
    tracing::debug!("json params {:?}", input);
    Html("<h3>Json posted</h3>")
}

/**
 * 解析错误处理请求
 * 想要处理请求的解析错误，可以使用 Axum 的 Rejection
 * 只需要在写解包器的时候，把参数类型改成使用 Result 包起来，Result 的错误类型为相应的解包器对应的 Rejection 类型就行了
 * 比如 Json 解包器就对应 JsonRejection，Form 解包器就对应 FormRejection
 */
async fn handle_parsing_error(payload: Result<Json<Input>, JsonRejection>) {
    match payload {
        Ok(payload) => {
            // 这里 payload 是一个有效的 JSON
            tracing::debug!("json params {:?}", payload);
        }
        Err(JsonRejection::MissingJsonContentType(_)) => {
            // 请求没有 `Content-Type: application/json` 头时
        }
        Err(JsonRejection::JsonDataError(_)) => {
            // 无法将 body 反序列化为目标类型
        }
        Err(JsonRejection::JsonSyntaxError(_)) => {
            // body 中语法错误
        }
        Err(JsonRejection::BytesRejection(_)) => {
            // 提取请求 body 失败
        }
        Err(_) => {
            // `JsonRejection` 标记为 `#[non_exhaustive]`，所以必须兜底
        }
    }
}

/**
 * Axum handler 返回值很灵活，只要实现了 IntoResponse 这个 trait 的类型，都能用作 handler 的返回值。
 * Axum 会根据返回值的类型，对 Http Response 的 status code 和 header 等进行自动配置，减少了开发者对细节的处理。
 */
async fn handler_return(Json(input): Json<Input>) -> impl IntoResponse {
    // 返回一个 HTML
    // Html("<h3>handler return</h3>")

    // 返回一个 String
    // "handler return"

    /*
     * 返回一个 Json
     * 在 Axum 里 Json 既是解包器，又可以用在 response 里面。
     * 借助 serde_json 提供的 json! 宏，可以方便地构造 Json 对象。
     */
    // Json(json!({ "result": "ok", "number": 1, }))

    // 返回一个 Redirect 自动重定向页面
    // Redirect::to("/")

    // 可以在 https://docs.rs/axum/latest/axum/response/trait.IntoResponse.html#foreign-impls 查看其他返回形式
    // (StatusCode::OK, "Hello, world!")

    /*
     * 注意，如果一个 handler 里需要返回两个或多个不同的类型，那么需要调用 .into_response() 转换一下。
     * impl trait 这种在函数中的写法，本质上仍然是编译期单态化，每次编译都会替换成一个具体的类型。
     */
    if input.name.is_empty() {
        Json(json!({ "result": "ok", "number": 1, })).into_response()
    } else {
        Redirect::to("/").into_response()
    }
}
