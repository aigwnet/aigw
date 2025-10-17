use dyn_fmt::AsStrFormatExt;
use lazy_static::lazy_static;

use http::StatusCode;

use crate::{version::VERSION, SERVER};

const ERROR_TEMPLATE: &str = r#"
<!doctype html>
<html lang="en" class="h-100" data-bs-theme="auto">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    <link href="/.__aigw__reserved/assets/dist/css/bootstrap.min.css" rel="stylesheet">
</head>
<body class="d-flex flex-column h-100">        
    <main class="w-100 m-auto">
        <div class="container">
            <div class="row justify-content-center">
                <h1 class="main" style="font-size: 20vw;">{}</h1>
                <p class="sub" style="font-size: 1.95vw;">{}</p>
            </div>
        </div>
    </main>
    <footer class="footer mt-auto py-3 bg-body-tertiary">
        <div class="container">
            <span class="text-body-secondary"> {}/{} </span>
        </div>
    </footer>
</body>
</html>"#;

lazy_static! {
    static ref ERR_400: String = ERROR_TEMPLATE.format(&["400 Bad Request", "400", "This page isn't available.", SERVER, VERSION]);
    static ref ERR_403: String = ERROR_TEMPLATE.format(&["403 Forbidden", "403", "This page isn't available.", SERVER, VERSION]);
    static ref ERR_404: String = ERROR_TEMPLATE.format(&["404 Not Found", "404", "Sorry but the page you are looking for does not exist, have been removed. name changed or is temporarily unavailable.", SERVER, VERSION]);
    static ref ERR_405: String = ERROR_TEMPLATE.format(&["405 Method Not Allowed", "405", "Your request could not be allowed.", SERVER, VERSION]);
    static ref ERR_500: String = ERROR_TEMPLATE.format(&["500 Internal Server Error", "500", "Oh eyeballs! Something went wrong. We're looking to see what happened.", SERVER, VERSION]);
    static ref ERR_DEFAULT: String = ERROR_TEMPLATE.format(&["Error", "Error", "Oh eyeballs! Something went wrong. We're looking to see what happened.", SERVER, VERSION]);
}

pub fn get_error_page(status: StatusCode) -> &'static str {
    if status == StatusCode::BAD_REQUEST {
        return ERR_400.as_str();
    } else if status == StatusCode::FORBIDDEN {
        return ERR_403.as_str();
    } else if status == StatusCode::NOT_FOUND {
        return ERR_404.as_str();
    } else if status == StatusCode::METHOD_NOT_ALLOWED {
        return ERR_405.as_str();
    } else if status == StatusCode::INTERNAL_SERVER_ERROR {
        return ERR_500.as_str();
    }
    ERR_DEFAULT.as_str()
}
