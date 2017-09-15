error_chain! {
    foreign_links {
        Io(::std::io::Error);
        Reqwest(::reqwest::Error);
    }

    links {
        CaSyncFormat(::casync_format::Error, ::casync_format::ErrorKind);
    }
}
