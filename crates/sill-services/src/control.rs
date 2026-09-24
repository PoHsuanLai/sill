//! A service's command channel: the half of a service that is not its watch receiver.

use tokio::sync::mpsc;

use crate::error::ServiceError;

/// Sends commands of type `C` to one running service. Cheap to clone; every surface holds one
/// through [`crate::Services`].
#[derive(Debug)]
pub struct Control<C> {
    tx: mpsc::UnboundedSender<C>,
}

impl<C> Clone for Control<C> {
    fn clone(&self) -> Self {
        Control {
            tx: self.tx.clone(),
        }
    }
}

impl<C> Control<C> {
    /// A control and the receiver its service task drains.
    pub fn channel() -> (Control<C>, mpsc::UnboundedReceiver<C>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Control { tx }, rx)
    }

    /// Queue `command`; `ServiceError::Gone` once the service has stopped.
    pub fn send(&self, command: C) -> Result<(), ServiceError> {
        self.tx.send(command).map_err(|_| ServiceError::Gone)
    }
}

#[cfg(test)]
mod tests {
    use super::Control;
    use crate::error::ServiceError;

    #[test]
    fn a_command_reaches_the_service_until_it_stops() {
        let (control, mut rx) = Control::<u8>::channel();
        let clone = control.clone();
        assert!(control.send(1).is_ok());
        assert!(clone.send(2).is_ok());
        assert_eq!(rx.try_recv().ok(), Some(1));
        assert_eq!(rx.try_recv().ok(), Some(2));
        drop(rx);
        assert!(matches!(control.send(3), Err(ServiceError::Gone)));
    }
}
