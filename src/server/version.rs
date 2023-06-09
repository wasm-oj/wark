use rocket::{fairing::AdHoc, http::Header};

pub fn fairing() -> AdHoc {
    AdHoc::on_response("Version Headers", |_req, res| {
        Box::pin(async move {
            res.set_header(Header::new(
                "X-Powered-By",
                "WARK (https://github.com/wasm-oj/wark)",
            ));
            res.set_header(Header::new("X-Version", env!("VERGEN_GIT_DESCRIBE")));
        })
    })
}
