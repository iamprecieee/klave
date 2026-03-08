use std::{convert::Infallible, time::Duration};

use axum::{
    Extension,
    extract::State,
    response::sse::{Event, Sse},
};
use futures_util::stream::Stream;
use tokio_stream::{StreamExt, wrappers::BroadcastStream};

use crate::{middleware::AuthContext, state::AppState};

pub async fn sse_handler(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.event_tx.subscribe();

    // Operators see all events; agents only see their own
    let filter_agent_id = if auth.is_operator {
        None
    } else {
        auth.agent_id.clone()
    };

    let stream = BroadcastStream::new(rx).filter_map(move |msg| {
        match msg {
            Ok(event) => {
                if let Some(ref filter_id) = filter_agent_id
                    && let Some(event_agent) = event.agent_id()
                    && event_agent != filter_id
                {
                    return None;
                }

                match serde_json::to_string(&event) {
                    Ok(json) => Some(Ok(Event::default().data(json))),
                    Err(_) => None,
                }
            }
            Err(_) => None, // Lagged or closed
        }
    });

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}
