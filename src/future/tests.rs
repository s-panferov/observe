use futures::stream::StreamExt;
use tokio::time::Duration;
use tracing_subscriber::{fmt::Subscriber, EnvFilter};

use crate::observable::{Observable, ObservableExt};
use crate::{EvalContext, Var};

use super::{ComputedFuture, FutureEffect, Tokio};
use std::task::Poll;

#[tokio::test]
async fn futured() {
    let logs = Subscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();

    tracing::subscriber::set_global_default(logs).unwrap();

    let fut_val = ComputedFuture::<_, Tokio>::new(move |_ctx: &mut EvalContext| async {
        tokio::time::delay_for(Duration::from_millis(200)).await;
        true
    });

    fut_val.set_name(String::from("Future"));

    let (rx, mut stream) = fut_val.stream::<Tokio>();

    rx.autorun();
    rx.update();

    println!("{:?}", stream.next().await);
    assert!(matches!(fut_val.once(), Poll::Pending));

    println!("{:?}", stream.next().await);
    assert!(matches!(fut_val.once(), Poll::Ready(true)));
}

#[tokio::test]
async fn effect() {
    let value = Var::new(true);
    let fut_val = ComputedFuture::<_, Tokio>::new(FutureEffect::new(
        move |ctx| value.get(ctx),
        move |var| async {
            tokio::time::delay_for(Duration::from_millis(200)).await;
            true
        },
    ));
}
