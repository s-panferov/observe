use super::*;
use crate::shared::Value;

use futures::stream::StreamExt;
use tokio::time::Duration;

use tracing_subscriber::{fmt::Subscriber, EnvFilter};

#[tokio::test]
async fn futured() {
    let logs = Subscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();

    tracing::subscriber::set_global_default(logs).unwrap();

    let fut_val = Value::from(TokioComputedFuture::new(
        move |_ctx: &mut EvalContext<_>| async {
            tokio::time::delay_for(Duration::from_millis(200)).await;
            true
        },
    ));

    fut_val.set_name(String::from("Future"));

    let (rx, mut stream) = fut_val.stream::<TokioRuntime>();

    rx.autorun();
    rx.update();

    println!("{:?}", stream.next().await);
    assert!(matches!(*fut_val.once(), Poll::Pending));

    println!("{:?}", stream.next().await);
    assert!(matches!(*fut_val.once(), Poll::Ready(true)));
}
