use std::io::Cursor;
use okapi::openapi3::Responses;
use rocket::http::{ContentType, Status};
use rocket::Response;
use rocket::response::Responder;
use rocket_okapi::gen::OpenApiGenerator;
use rocket_okapi::response::OpenApiResponder;
pub use self::material::Instance;

mod domain_value;
mod dto;
mod material;
mod tools;
pub mod transfer;

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct GzippedResponse(Vec<u8>);


impl<'r> Responder<'r> for GzippedResponse {
    fn respond_to(self, _: &rocket::Request<'_>) -> rocket::response::Result<'r> {
        let mut response = Response::build();
        response
            .header(ContentType::JSON)
            .raw_header("Content-Encoding", "gzip")
            .status(Status::Ok)
            .sized_body(Cursor::new(self.0));
        response.ok()
    }
}


impl<'r> OpenApiResponder<'r> for GzippedResponse {
    fn responses(gen: &mut OpenApiGenerator) -> rocket_okapi::Result<Responses> {
        <Vec<u8> as OpenApiResponder>::responses(gen)
    }
}
