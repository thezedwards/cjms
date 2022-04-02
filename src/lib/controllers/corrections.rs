use actix_web::{web, HttpResponse};
use sqlx::PgPool;

use crate::settings::Settings;

pub async fn detail(
    path: web::Path<String>,
    settings: web::Data<Settings>,
    _pool: web::Data<PgPool>,
) -> HttpResponse {
    if !path.into_inner().eq(&settings.authentication) {
        return HttpResponse::NotFound().finish();
    }
    HttpResponse::Ok().finish()
}
