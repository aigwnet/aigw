use bytes::Bytes;
use dyn_fmt::AsStrFormatExt;
use lazy_static::lazy_static;

use http::{StatusCode, header};
use pingora_http::ResponseHeader;

use crate::{SERVER, version::VERSION};

const ERROR_TEMPLATE: &str = r##"
<!DOCTYPE html>
<html lang="en-US">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}

        body {{
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            display: flex;
            justify-content: center;
            align-items: center;
            color: #333;
        }}

        .error-container {{
            text-align: center;
            max-width: 600px;
            padding: 2rem;
            background: white;
            border-radius: 20px;
            box-shadow: 0 20px 40px rgba(0, 0, 0, 0.1);
            animation: fadeIn 0.8s ease-out;
        }}

        .error-code {{
            font-size: 10rem;
            font-weight: 800;
            color: #667eea;
            line-height: 1;
            margin-bottom: 1rem;
            text-shadow: 4px 4px 0px rgba(102, 126, 234, 0.2);
        }}

        .error-title {{
            font-size: 2.5rem;
            font-weight: 700;
            margin-bottom: 1rem;
            color: #2d3748;
        }}

        .error-message {{
            font-size: 1.2rem;
            color: #718096;
            margin-bottom: 2rem;
            line-height: 1.6;
        }}

        .server-info {{
            font-size: 0.9rem;
            color: #a0aec0;
            margin: 1.5rem 0;
            font-family: 'Courier New', monospace;
        }}

        .illustration {{
            margin: 2rem 0;
            opacity: 0.8;
        }}

        .illustration svg {{
            width: 200px;
            height: 200px;
            margin: 0 auto;
        }}

        @keyframes fadeIn {{
            from {{
                opacity: 0;
                transform: translateY(20px);
            }}
            to {{
                opacity: 1;
                transform: translateY(0);
            }}
        }}

        @media (max-width: 768px) {{
            .error-code {{
                font-size: 6rem;
            }}
            
            .error-title {{
                font-size: 2rem;
            }}
            
            .error-message {{
                font-size: 1rem;
            }}
            
            .server-info {{
                font-size: 0.8rem;
            }}
            
            .error-container {{
                margin: 1rem;
                padding: 1.5rem;
            }}
        }}

        @media (max-width: 480px) {{
            .error-code {{
                font-size: 4rem;
            }}
            
            .error-title {{
                font-size: 1.5rem;
            }}
            
            .illustration svg {{
                width: 150px;
                height: 150px;
            }}
        }}
    </style>
</head>
<body>
    <div class="error-container">
        <div class="error-code">{}</div>
        <h1 class="error-title">{}</h1>
        <p class="error-message">{}</p>
        
        <div class="illustration">
            <svg viewBox="0 0 200 200" xmlns="http://www.w3.org/2000/svg">
                <path fill="#667eea" d="M40,40 L160,40 L160,160 L40,160 Z" stroke="#764ba2" stroke-width="8" fill-opacity="0.1"/>
                <circle cx="100" cy="100" r="40" fill="#667eea" fill-opacity="0.2"/>
                <path d="M80,80 L120,120 M120,80 L80,120" stroke="#764ba2" stroke-width="8" stroke-linecap="round"/>
            </svg>
        </div>
        <div class="server-info">{}/{}</div>
    </div>

</body>
</html>
"##;

lazy_static! {
    static ref ERR_400: String = ERROR_TEMPLATE.format(&["400 Bad Request", "400", "Bad Request", "This page isn't available.", SERVER, VERSION]);
    static ref ERR_403: String = ERROR_TEMPLATE.format(&["403 Forbidden", "403", "Forbidden", "This page isn't available.", SERVER, VERSION]);
    static ref ERR_404: String = ERROR_TEMPLATE.format(&["404 Not Found", "404", "Not Found", "Sorry but the page you are looking for does not exist, have been removed. name changed or is temporarily unavailable.", SERVER, VERSION]);
    static ref ERR_405: String = ERROR_TEMPLATE.format(&["405 Method Not Allowed", "405", "Method Not Allowed", "Your request could not be allowed.", SERVER, VERSION]);
    static ref ERR_429: String = ERROR_TEMPLATE.format(&["429 Too Many Requests", "429", "Too Many Requests", "Your request could not be allowed.", SERVER, VERSION]);
    static ref ERR_500: String = ERROR_TEMPLATE.format(&["500 Internal Server Error", "500", "Internal Server Error", "Oh eyeballs! Something went wrong. We're looking to see what happened.", SERVER, VERSION]);
    static ref ERR_502: String = ERROR_TEMPLATE.format(&["502 Bad Gateway", "502", "Bad Gateway", "Server Error! The server encountered a temporary error and could not complete your request.", SERVER, VERSION]);
    static ref ERR_DEFAULT: String = ERROR_TEMPLATE.format(&["Error", "Error", "Error", "Oh eyeballs! Something went wrong. We're looking to see what happened.", SERVER, VERSION]);
}

pub fn get_error_page(status: StatusCode) -> &'static [u8] {
    if status == StatusCode::BAD_REQUEST {
        return ERR_400.as_str().as_bytes();
    } else if status == StatusCode::FORBIDDEN {
        return ERR_403.as_str().as_bytes();
    } else if status == StatusCode::NOT_FOUND {
        return ERR_404.as_str().as_bytes();
    } else if status == StatusCode::METHOD_NOT_ALLOWED {
        return ERR_405.as_str().as_bytes();
    } else if status == StatusCode::TOO_MANY_REQUESTS {
        return ERR_429.as_str().as_bytes();
    } else if status == StatusCode::INTERNAL_SERVER_ERROR {
        return ERR_500.as_str().as_bytes();
    } else if status == StatusCode::BAD_GATEWAY {
        return ERR_502.as_str().as_bytes();
    }
    ERR_DEFAULT.as_str().as_bytes()
}

pub fn generate_error(code: StatusCode) -> (ResponseHeader, Bytes) {
    let body = Bytes::from_static(get_error_page(code));

    let length = body.len();
    let mut resp = ResponseHeader::build(code, Some(3)).unwrap();
    resp.insert_header(header::SERVER, SERVER).unwrap();
    resp.insert_header(header::CONTENT_LENGTH, length.to_string())
        .unwrap();
    resp.insert_header(header::CACHE_CONTROL, "private, no-store")
        .unwrap();
    (resp, body)
}
