error_chain! {
    links {
        Format(::casync_format::Error, ::casync_format::ErrorKind);
    }

    foreign_links {
        Utf8(::std::string::FromUtf8Error);
        Io(::std::io::Error);
    }
}
