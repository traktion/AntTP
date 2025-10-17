use std::time::{Duration, SystemTime};
use actix_files::file_extension_to_mime;
use actix_http::header;
use actix_web::http::header::{CacheControl, CacheDirective, ContentLength, ContentRange, ContentRangeSpec, ContentType, ETag, EntityTag, Expires, HeaderName};
use mime::Mime;
use xor_name::XorName;
use crate::config::anttp_config::AntTpConfig;
use crate::service::resolver_service::ResolverService;

pub struct HeaderBuilder {
    resolver_service: ResolverService,
    ant_tp_config: AntTpConfig
}

impl HeaderBuilder {
    
    pub fn new(resolver_service: ResolverService, ant_tp_config: AntTpConfig) -> Self {
        Self { resolver_service, ant_tp_config }
    }
    
    pub fn build_cache_control_header(&self, xor_name: &XorName, is_resolved_file_name: bool) -> CacheControl {
        if !is_resolved_file_name && self.resolver_service.is_immutable_address(&format!("{:x}", xor_name)) {
            CacheControl(vec![CacheDirective::MaxAge(u32::MAX), CacheDirective::Public]) // immutable
        } else {
            CacheControl(vec![CacheDirective::MaxAge(u32::try_from(self.ant_tp_config.cached_mutable_ttl).unwrap()), CacheDirective::Public]) // mutable
        }
    }

    pub fn build_expires_header(&self, xor_name: &XorName, is_resolved_file_name: bool) -> Expires {
        if !is_resolved_file_name && self.resolver_service.is_immutable_address(&format!("{:x}", xor_name)) {
            Expires((SystemTime::now() + Duration::from_secs(u64::from(u32::MAX))).into()) // immutable
        } else {
            Expires((SystemTime::now() + Duration::from_secs(self.ant_tp_config.cached_mutable_ttl)).into()) // mutable
        }
    }

    pub fn build_content_type_header(&self, extension: &str) -> ContentType {
        // todo: remove markdown exclusion when IMIM fixed
        if extension != "" && extension != "md" {
            ContentType(file_extension_to_mime(extension))
        } else {
            ContentType(mime::TEXT_HTML) // default to text/html
        }
    }

    pub fn build_content_type_header_from_mime(&self, mime: Mime) -> ContentType {
        ContentType(mime)
    }
    
    pub fn build_etag_header(&self, xor_name: &XorName) -> ETag {
        ETag(EntityTag::new_strong(format!("{:x}", xor_name).to_owned()))
    }

    pub fn build_cors_header(&self) -> (HeaderName, &str) {
        (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
    }

    pub fn build_server_header(&self) -> (HeaderName, String) {
        (header::SERVER, format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")))
    }
    
    pub fn build_content_range_header(&self, range_from: u64, range_to: u64, content_length: u64) -> ContentRange {
        ContentRange(ContentRangeSpec::Bytes { range: Some((range_from, range_to)), instance_length: Some(content_length) })
    }
    
    pub fn build_accept_ranges_header(&self) -> (HeaderName, &str) {
        (header::ACCEPT_RANGES, "bytes")
    }
    
    pub fn build_content_length_header(&self, content_length: u64) -> ContentLength {
        ContentLength(usize::try_from(content_length).unwrap())
    }
    
    pub fn build_location_header(&self, path: String) -> (HeaderName, String) {
        (header::LOCATION, path.to_string())
    }
}