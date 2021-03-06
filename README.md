# flo-task

Async task utilities.

## `SpawnScope`

RAII guard used to notify child tasks that the parent has been dropped.
One use-case of this is to implement gracefully shut down a group of child tasks.

```rust
#[tokio::test]
async fn test_initial_value() {
    use std::future::Future;
    use std::time::Duration;
    use tokio::time::delay_for;
    let scope = SpawnScope::new();

    fn get_task(mut scope: SpawnScopeHandle) -> impl Future<Output = i32> {
        async move {
            let mut n = 0;
            loop {
                tokio::select! {
                  _ = scope.left() => {
                    return n
                  }
                  _ = delay_for(Duration::from_millis(50)) => {
                    n = n + 1
                  }
                }
            }
        }
    }

    let t1 = tokio::spawn(get_task(scope.handle()));
    let t2 = tokio::spawn(get_task(scope.handle()));
    let t3 = tokio::spawn(get_task(scope.handle()));

    delay_for(Duration::from_millis(100)).await;
    drop(scope);

    let (v1, v2, v3) = tokio::try_join!(t1, t2, t3).unwrap();
    assert!(v1 > 0);
    assert!(v2 > 0);
    assert!(v3 > 0);
}
```
