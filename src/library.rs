#[macro_export]
macro_rules! express {
    ($receiver:ident $(.$method:ident($($args:tt)*))*) => {
        {
            let mut receiver = $receiver;
            $(
                receiver.$method($($args)*);
            )*
            receiver
        }
    }
}
