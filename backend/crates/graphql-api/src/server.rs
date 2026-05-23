use async_graphql::Schema;
use async_graphql_axum::{GraphQLRequest, GraphQLResponse, GraphQLProtocol, GraphQLWebSocket};
use axum::{
    extract::State,
    routing::{get, post},
    Router,
};
use crate::{
    context::GraphQlContext,
    mutation::MutationRoot,
    query::QueryRoot,
    subscription::SubscriptionRoot,
};

pub type DraoxSchema = Schema<QueryRoot, MutationRoot, SubscriptionRoot>;

/// Build the async-graphql schema, injecting services via `ctx`.
pub fn build_schema(ctx: GraphQlContext) -> DraoxSchema {
    Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
        .data(ctx)
        .finish()
}

/// Create an axum [`Router`] that serves:
/// - `POST /graphql` — query & mutation
/// - `GET  /graphql` — GraphiQL playground (development)
/// - `GET  /graphql/ws` — WebSocket subscriptions
pub fn graphql_router(schema: DraoxSchema) -> Router {
    Router::new()
        .route("/graphql", post(graphql_handler).get(graphiql_handler))
        .route("/graphql/ws", get(ws_handler))
        .with_state(schema)
}

async fn graphql_handler(
    State(schema): State<DraoxSchema>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

async fn graphiql_handler() -> axum::response::Html<String> {
    axum::response::Html(
        async_graphql::http::GraphiQLSource::build()
            .endpoint("/graphql")
            .subscription_endpoint("/graphql/ws")
            .finish(),
    )
}

async fn ws_handler(
    State(schema): State<DraoxSchema>,
    ws: axum::extract::WebSocketUpgrade,
    protocol: GraphQLProtocol,
) -> impl axum::response::IntoResponse {
    ws.on_upgrade(move |socket| {
        GraphQLWebSocket::new(socket, schema, protocol).serve()
    })
}
