fn main() {
    std::fs::read_to_string(".env")
        .into_iter()
        .flat_map(|s| {
            s.lines()
                .map(str::trim)
                .filter(|s| !s.starts_with('#'))
                .flat_map(|s| {
                    s.split_once('=')
                        .map(|(k, v)| std::env::set_var(k, v.replace("\"", "")))
                })
                .last()
        })
        .last()
        .and_then(|_| {
            std::iter::once(
                [
                    ("DSC_PASS", Option::<String>::None),
                    ("DSC_NICK", Option::<String>::None),
                    ("DSC_CHANNEL", Option::<String>::None),
                ]
                .into_iter()
                .flat_map(|(key, mut val)| {
                    std::env::var(key)
                        .ok()
                        .map(|v| val.get_or_insert(v).clone())
                        .map(|v| (key, v))
                })
                .collect::<std::collections::HashMap<_, _>>(),
            )
            .next()
        })
        .and_then(|config| {
            std::net::TcpStream::connect("irc.chat.twitch.tv:6667")
                .ok()
                .and_then(|stream| {
                    [
                        ("DSC_PASS", "PASS"),
                        ("DSC_NICK", "NICK"),
                        ("DSC_CHANNEL", "JOIN"),
                    ]
                    .into_iter()
                    .flat_map(|(key, cmd)| {
                        std::io::Write::write_all(
                            &mut &stream,
                            format!(
                                "{} {}\r\n",
                                cmd,
                                config
                                    .get(key)
                                    .unwrap_or_else(|| panic!("{} must be set", key)),
                            )
                            .as_bytes(),
                        )
                    })
                    .flat_map(|_| std::io::Write::flush(&mut &stream))
                    .last()
                    .map(|_| stream)
                })
        })
        .and_then(|stream| {
            std::io::BufRead::lines(std::io::BufReader::new(&stream))
                .flatten()
                .map(|line| {
                    [
                        (|line, _stream| println!(r"<< {}\r\n", line))
                            as fn(&str, &std::net::TcpStream),
                        (|line, mut stream| {
                            line.starts_with("PING")
                                .then(|| {
                                    std::io::Write::write_all(
                                        &mut stream,
                                        line.replace("PING", "PONG").as_bytes(),
                                    )
                                })
                                .map(drop)
                                .unwrap_or_default()
                        }),
                        (|line, stream| {
                            line.splitn(4, ' ')
                                .skip_while(|&s| s != "PRIVMSG")
                                .nth(2)
                                .and_then(|s| s.strip_prefix(':'))
                                .map(|data| {
                                    (
                                        data,
                                        &[(|line, mut stream| {
                                            std::io::Write::write_all(&mut stream, line.as_bytes())
                                                .map(|_| {
                                                    std::io::Write::write_all(&mut stream, b"\r\n")
                                                })
                                                .ok()
                                                .map(|_| println!(r">> {}\r\n", line))
                                                .unwrap_or_default()
                                        })
                                            as fn(&str, &std::net::TcpStream)],
                                    )
                                })
                                .and_then(|(data, funcs)| {
                                    <&[(
                                        &str,
                                        fn(
                                            &str,
                                            &str,
                                            &str,
                                            &std::net::TcpStream,
                                            &[fn(&str, &std::net::TcpStream)],
                                        ),
                                    )]>::into_iter(&[
                                        (
                                            "!hello",
                                            (|channel, nick, _data, stream, funcs| {
                                                funcs[0](
                                                    &format!(
                                                        "PRIVMSG {} :hello {}!",
                                                        channel, nick
                                                    ),
                                                    stream,
                                                )
                                            }),
                                        ),
                                        (
                                            "!source",
                                            (|channel, nick, _data, stream, funcs| {
                                                funcs[0](
                                                    &format!(
                                                        "PRIVMSG {} :{}: {}",
                                                        channel,
                                                        nick,
                                                        "you can view this\
                                                         monstrosity^wsimplicity \
                                                         at https://github.com/\
                                                         museun/diet-semicola/\
                                                         blob/main/src/main.rs"
                                                    ),
                                                    stream,
                                                )
                                            }),
                                        ),
                                        (
                                            "!project",
                                            (|channel, nick, _data, stream, funcs| {
                                                funcs[0](
                                                    &format!(
                                                        "PRIVMSG {} :{}: {}",
                                                        channel,
                                                        nick,
                                                        "consider using a semicolon here: `\x3b`"
                                                    ),
                                                    stream,
                                                )
                                            }),
                                        ),
                                    ])
                                    .flat_map(|(cmd, func)| {
                                        line.split_once('!')
                                            .and_then(|(head, _)| {
                                                line.find("PRIVMSG ")
                                                    .and_then(|index| {
                                                        line[index + "PRIVMSG ".len()..]
                                                            .split_once(' ')
                                                            .map(|(head, _)| head)
                                                    })
                                                    .and_then(|channel| {
                                                        head.strip_prefix(':')
                                                            .map(|nick| (nick, channel))
                                                    })
                                            })
                                            .and_then(|(nick, channel)| {
                                                data.split_once(' ')
                                                    .map(|(head, _)| head)
                                                    .or(Some(data))
                                                    .filter(|head| head == cmd)
                                                    .map(|_| {
                                                        func(channel, nick, data, &stream, funcs)
                                                    })
                                            })
                                    })
                                    .last()
                                })
                                .unwrap_or_default()
                        }),
                    ]
                    .into_iter()
                    .zip(std::iter::repeat(&*line))
                    .map(|(f, s)| f(&*s, &stream))
                    .flat_map(|_| std::io::Write::flush(&mut &stream).ok())
                    .last()
                })
                .map(drop)
                .last()
        })
        .unwrap_or_default()
}
