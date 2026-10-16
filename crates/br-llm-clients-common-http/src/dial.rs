use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::time::Duration;

use tokio::task::JoinSet;
use tokio::time::sleep;

pub(crate) async fn first_connected<T, F, Fut>(
    addresses: Vec<SocketAddr>,
    stagger: Duration,
    attempt: F,
) -> io::Result<T>
where
    T: Send + 'static,
    F: Fn(SocketAddr) -> Fut,
    Fut: Future<Output = io::Result<T>> + Send + 'static,
{
    let mut remaining = addresses.into_iter();
    let mut running = JoinSet::new();
    let mut last_error: Option<io::Error> = None;
    let mut start_next = true;
    loop {
        if start_next {
            if let Some(address) = remaining.next() {
                running.spawn(attempt(address));
            }
            start_next = false;
        }
        if running.is_empty() {
            return Err(last_error.unwrap_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, "no address to connect to")
            }));
        }
        let more = remaining.len() > 0;
        tokio::select! {
            joined = running.join_next() => match joined {
                Some(Ok(Ok(connected))) => return Ok(connected),
                Some(Ok(Err(error))) => {
                    last_error = Some(error);
                    start_next = true;
                }
                Some(Err(_)) => {
                    last_error = Some(io::Error::other("connect attempt aborted"));
                    start_next = true;
                }
                None => {}
            },
            () = sleep(stagger), if more => start_next = true,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use tokio::time::Instant;

    use super::*;

    const STAGGER: Duration = crate::tuning::Tuning::FIXED.dial_stagger;

    fn addresses(count: u8) -> Vec<SocketAddr> {
        (1..=count)
            .map(|n| SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, n)), 443))
            .collect()
    }

    #[tokio::test(start_paused = true)]
    async fn given_a_first_address_that_never_answers_when_connecting_then_the_second_connects_after_the_stagger()
     {
        let started = Instant::now();
        let connected = first_connected(addresses(2), STAGGER, |address| async move {
            if address.ip() == IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)) {
                std::future::pending::<()>().await;
            }
            Ok(address)
        })
        .await
        .expect("connected");
        assert_eq!(connected.ip(), IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)));
        assert_eq!(started.elapsed(), STAGGER);
    }

    #[tokio::test(start_paused = true)]
    async fn given_a_first_address_that_fails_fast_when_connecting_then_the_second_starts_at_once()
    {
        let started = Instant::now();
        let connected = first_connected(addresses(2), STAGGER, |address| async move {
            if address.ip() == IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)) {
                return Err(io::Error::from(io::ErrorKind::ConnectionRefused));
            }
            Ok(address)
        })
        .await
        .expect("connected");
        assert_eq!(connected.ip(), IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)));
        assert_eq!(started.elapsed(), Duration::ZERO);
    }

    #[tokio::test(start_paused = true)]
    async fn given_attempts_that_all_fail_when_connecting_then_the_last_error_is_returned() {
        let error = first_connected::<(), _, _>(addresses(3), STAGGER, |_| async {
            Err(io::Error::from(io::ErrorKind::ConnectionRefused))
        })
        .await
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::ConnectionRefused);
    }

    #[tokio::test(start_paused = true)]
    async fn given_no_address_when_connecting_then_it_fails() {
        let error = first_connected::<(), _, _>(Vec::new(), STAGGER, |_| async { Ok(()) })
            .await
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
    }

    #[tokio::test(start_paused = true)]
    async fn given_a_winner_when_connecting_then_attempts_still_running_are_dropped() {
        let started = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&started);
        let connected = first_connected(addresses(3), STAGGER, move |address| {
            let counter = Arc::clone(&counter);
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                if address.ip() == IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)) {
                    return Ok(address);
                }
                std::future::pending::<()>().await;
                Ok(address)
            }
        })
        .await
        .expect("connected");
        assert_eq!(connected.ip(), IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)));
        assert_eq!(started.load(Ordering::SeqCst), 2);
        tokio::time::sleep(STAGGER * 4).await;
        assert_eq!(started.load(Ordering::SeqCst), 2);
    }
}
