use heck::{ToUpperCamelCase, ToSnakeCase};
use openapiv3::{OpenAPI, ReferenceOr, Schema, SchemaKind, Type as OApiType};

/// A simplified intermediate representation used by all language emitters.
#[derive(Debug, Clone)]
pub struct ApiEndpoint {
    pub method: String,
    pub path: String,
    pub operation_id: String,
    pub summary: Option<String>,
    pub params: Vec<ApiParam>,
    pub request_body: Option<String>,
    pub response_type: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ApiParam {
    pub name: String,
    pub location: ParamLocation,
    pub required: bool,
    pub type_hint: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParamLocation {
    Path,
    Query,
    Header,
}

/// Extract a flat list of endpoints from an OpenAPI spec.
pub fn extract_endpoints(spec: &OpenAPI) -> Vec<ApiEndpoint> {
    let mut endpoints = Vec::new();

    for (path, item) in &spec.paths.paths {
        let item = match item {
            ReferenceOr::Item(i) => i,
            ReferenceOr::Reference { .. } => continue,
        };

        let ops = [
            ("GET", item.get.as_ref()),
            ("POST", item.post.as_ref()),
            ("PUT", item.put.as_ref()),
            ("PATCH", item.patch.as_ref()),
            ("DELETE", item.delete.as_ref()),
        ];

        for (method, maybe_op) in ops {
            let op = match maybe_op {
                Some(o) => o,
                None => continue,
            };

            let operation_id = op
                .operation_id
                .clone()
                .unwrap_or_else(|| default_op_id(method, path));

            let mut params = Vec::new();
            for p in &op.parameters {
                if let ReferenceOr::Item(param) = p {
                    let data = param.parameter_data_ref();
                    params.push(ApiParam {
                        name: data.name.clone(),
                        location: match param {
                            openapiv3::Parameter::Path { .. } => ParamLocation::Path,
                            openapiv3::Parameter::Header { .. } => ParamLocation::Header,
                            _ => ParamLocation::Query,
                        },
                        required: data.required,
                        type_hint: "string".into(),
                    });
                }
            }

            endpoints.push(ApiEndpoint {
                method: method.to_owned(),
                path: path.clone(),
                operation_id: operation_id.to_upper_camel_case(),
                summary: op.summary.clone(),
                params,
                request_body: op.request_body.as_ref().map(|_| "RequestBody".into()),
                response_type: Some("Response".into()),
                tags: op.tags.clone(),
            });
        }
    }

    endpoints
}

fn default_op_id(method: &str, path: &str) -> String {
    let slug = path
        .replace('/', "_")
        .replace('{', "")
        .replace('}', "")
        .to_snake_case();
    format!("{}_{}", method.to_lowercase(), slug)
}

/// Schema name from a `$ref` like `#/components/schemas/User`.
pub fn ref_name(reference: &str) -> String {
    reference.rsplit('/').next().unwrap_or(reference).to_owned()
}
