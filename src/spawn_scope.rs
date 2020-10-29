//! RAII guard used to notify child tasks that the parent has been dropped.

use std::future::Future;
use tokio::sync::watch::{channel, Receiver, Sender};

#[derive(Debug)]
pub struct SpawnScope {
    tx: Option<Sender<()>>,
    rx: Receiver<()>,
}

impl SpawnScope {
    pub fn new() -> Self {
        let (tx, rx) = channel(());
        Self { tx: Some(tx), rx }
    }

    pub fn handle(&self) -> SpawnScopeHandle {
        let rx = self.rx.clone();
        SpawnScopeHandle(rx)
    }

    pub fn spawn<F>(&self, future: F) 
    where F: Future<Output = ()> + Send + 'static
    {
        let mut handle = self.handle();
        tokio::spawn(async move {
            tokio::select! {
                _ = handle.left() => {},
                _ = future => {},
            }
        });
    }

    pub fn close(&mut self) {
        self.tx.take();
    }
}

#[derive(Debug)]
pub struct SpawnScopeHandle(Receiver<()>);

impl Clone for SpawnScopeHandle {
    fn clone(&self) -> Self {
        let rx = self.0.clone();
        SpawnScopeHandle(rx)
    }
}

impl SpawnScopeHandle {
    pub async fn left(&mut self) {
        while let Some(_) = self.0.recv().await {}
    }


    pub fn spawn<F>(&self, future: F) 
    where F: Future<Output = ()> + Send + 'static
    {
        let mut handle = self.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = handle.left() => {},
                _ = future => {},
            }
        });
    }
}

#[tokio::test]
async fn test_drop() {
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

#[tokio::test]
async fn test_spawn() {
    use tokio::sync::oneshot::*;
    
    let (tx, rx) = channel();

    struct Guard(Option<Sender<()>>);
    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.take().unwrap().send(()).ok();
        }
    }

    let g = Guard(tx.into());
    let scope = SpawnScope::new();

    scope.spawn(async move {
        futures::future::pending::<()>().await;
        drop(g)
    });

    drop(scope);

    rx.await.unwrap();
}