error_chain! {
    foreign_links {
        Utf8(::std::string::FromUtf8Error);
        Io(::std::io::Error);
    }
}
