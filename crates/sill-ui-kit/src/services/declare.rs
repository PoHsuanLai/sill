//! `services!`: the one declaration of every service. From each `field: Service = deps` line it
//! writes the [`Services`](super::Services) field, the [`Member`](super::Member) impl, the spawn
//! (in declaration order, so a service's `deps` can only name the ones declared above it), and,
//! behind `harness`, the fake channel. A service is added here and nowhere else.

/// The state a fake service starts at: the one given with `=>`, else its default.
#[cfg(feature = "harness")]
macro_rules! fake_state {
    ($service:ty) => {
        <<$service as sill_services::Service>::State as Default>::default()
    };
    ($service:ty, $fake:expr) => {
        $fake
    };
}

/// Declare the services, in the order they start.
///
/// ```ignore
/// services! {
///     start(rt, env, wiring: Wiring);
///     {
///         /// Docs of the field.
///         battery: BatteryService = ();                       // deps; the fake starts at its default
///         session: SessionService = SessionLinks { .. } => fake_session();   // deps => fake state
///     }
///     {
///         dirs: Dirs = env.dirs.clone() => Dirs::under(..);   // the fields that are not services
///     }
/// }
/// ```
///
/// `deps` is an expression that may name `rt`, `env`, `wiring` and the fields declared above it.
macro_rules! services {
    (
        start($rt:ident, $env:ident, $wiring:ident: $wiring_ty:ty);
        {
            $(
                $(#[$meta:meta])*
                $field:ident: $service:ty = $deps:expr $(=> $fake:expr)?;
            )*
        }
        {
            $(
                $(#[$xmeta:meta])*
                $xfield:ident: $xty:ty = $xstart:expr => $xfake:expr;
            )*
        }
    ) => {
        /// Every running service, each field named after its module. Surfaces read a service
        /// through `sill-ui-kit::hooks::use_service` and command it through `use_control`;
        /// nothing here is a Dioxus signal (review rule: never pass a Signal through
        /// `Root::context`).
        #[derive(Debug, Clone)]
        pub struct Services {
            $(
                $(#[$meta])*
                pub $field: sill_services::Handle<$service>,
            )*
            $(
                $(#[$xmeta])*
                pub $xfield: $xty,
            )*
        }

        $(
            impl super::Member for $service {
                fn of(services: &Services) -> &sill_services::Handle<Self> {
                    &services.$field
                }
            }
        )*

        impl Services {
            /// Each declared service as `(field, name)`, in start order.
            pub const DECLARED: &'static [(&'static str, &'static str)] =
                &[$((stringify!($field), <$service as sill_services::Service>::NAME)),*];

            /// Start every service on `rt` against `env`'s seams, each after the ones it depends
            /// on. A service whose system half is missing still yields its receiver, at its
            /// default.
            pub fn start(
                $rt: &tokio::runtime::Handle,
                $env: &sill_services::env::Env,
                $wiring: $wiring_ty,
            ) -> Services {
                $(let $field = sill_services::service::spawn::<$service>($rt, $env, $deps);)*
                Services {
                    $($field,)*
                    $($xfield: $xstart,)*
                }
            }

            /// Each service this value holds, as `(field, name)`: [`Self::DECLARED`] when every
            /// declared service was started or faked.
            pub fn held(&self) -> Vec<(&'static str, &'static str)> {
                vec![$({
                    let _held: &sill_services::Handle<$service> = &self.$field;
                    (stringify!($field), <$service as sill_services::Service>::NAME)
                }),*]
            }

            /// Every service as a fake: its channel at the state given with `=>` (else its
            /// default), and the service's halves of the channels, for a test to claim.
            #[cfg(feature = "harness")]
            pub(crate) fn fake() -> (Services, Ios) {
                struct Pairs {
                    $($field: (
                        sill_services::Handle<$service>,
                        sill_services::ServiceIo<$service>,
                    ),)*
                }
                let pairs = Pairs {
                    $($field: sill_services::Handle::<$service>::channel(
                        fake_state!($service $(, $fake)?),
                    ),)*
                };
                let services = Services {
                    $($field: pairs.$field.0,)*
                    $($xfield: $xfake,)*
                };
                let ios = Ios {
                    $($field: super::fake::Claim::new(pairs.$field.1),)*
                };
                (services, ios)
            }
        }

        /// The service halves of every fake channel.
        #[cfg(feature = "harness")]
        pub(crate) struct Ios {
            $(pub(crate) $field: super::fake::Claim<$service>,)*
        }

        #[cfg(feature = "harness")]
        impl Ios {
            /// Keep every half nobody claimed alive in `keep`, so sends to it do not fail.
            pub(crate) fn keep_unclaimed(self, keep: &mut Vec<Box<dyn std::any::Any>>) {
                $(self.$field.keep(keep);)*
            }
        }
    };
}
