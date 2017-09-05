error_chain! {
    links {
        Format(::casync_format::Error, ::casync_format::ErrorKind);
    }

    foreign_links {
        Io(::std::io::Error);
    }
}
