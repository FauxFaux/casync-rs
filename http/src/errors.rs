error_chain! {
    foreign_links {
        Io(::std::io::Error);
        HyperUri(::hyper::error::UriError);
        Hyper(::hyper::Error);
    }
}
