fn main() {
    std::fs::read_to_string(".env")
        .into_iter()
        .flat_map(|s| {
            s.lines()
                .map(str::trim)
                .filter(|s| !s.starts_with('#'))
                .flat_map(|s| s.split_once('=').map(|(k, v)| std::env::set_var(k, v.replace("\"", ""))))
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
                .flat_map(|(key, mut val)| std::env::var(key).ok().map(|v| val.get_or_insert(v).clone()).map(|v| (key, v)))
                .collect::<std::collections::HashMap<_, _>>(),
            )
            .next()
        })
        .and_then(|config| {
            std::net::TcpStream::connect("irc.chat.twitch.tv:6667").ok().and_then(|stream| {
                [("DSC_PASS", "PASS"), ("DSC_NICK", "NICK"), ("DSC_CHANNEL", "JOIN")]
                    .into_iter()
                    .flat_map(|(key, cmd)| {
                        std::io::Write::write_all(
                            &mut &stream,
                            format!("{} {}\r\n", cmd, config.get(key).unwrap_or_else(|| panic!("{} must be set", key)),).as_bytes(),
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
                        (|line, _stream| println!(r"<< {}\r\n", line)) as fn(&str, &std::net::TcpStream),
                        (|line, mut stream| {
                            line.starts_with("PING")
                                .then(|| std::io::Write::write_all(&mut stream, line.replace("PING", "PONG").as_bytes()))
                                .map(drop)
                                .unwrap_or_default()
                        }),
                        (|line, stream| {
                            line.splitn(4, ' ')
                                .skip_while(|&s| s != "PRIVMSG")
                                .nth(2)
                                .and_then(|s| s.strip_prefix(':'))
                                .and_then(|data| {
                                    line.split_once('!')
                                        .and_then(|(head, _)| {
                                            line.find("PRIVMSG ")
                                                .and_then(|index| line[index + "PRIVMSG ".len()..].split_once(' ').map(|(head, _)| head))
                                                .and_then(|channel| head.strip_prefix(':').map(|nick| (nick, channel, data)))
                                        })
                                        .map(|(nick, channel, data)| {
                                            [("nick", nick), ("channel", channel), ("input", data)]
                                                .into_iter()
                                                .map(|(k, v)| (k, std::borrow::Cow::from(v)))
                                                .collect::<std::collections::HashMap<&'static str, std::borrow::Cow<'_, str>>>()
                                        })
                                })
                                .map(|obj| {
                                    (obj, |raw: &str, mut stream: &std::net::TcpStream| {
                                        std::io::Write::write_all(&mut stream, raw.as_bytes())
                                            .map(|_| std::io::Write::write_all(&mut stream, b"\r\n"))
                                            .map(|_| std::io::Write::flush(&mut stream))
                                            .map(|_| println!(r">> {}\r\n", raw))
                                            .ok()
                                            .unwrap_or_default()
                                    })
                                })
                                .map(|(obj, write)| {
                                    (
                                        obj,
                                        write,
                                        [
                                            (
                                                "raw",
                                                (|obj, write, stream| obj.get("raw").map(|raw| write(raw, stream)).unwrap_or_default())
                                                    as fn(
                                                        std::collections::HashMap<&'static str, std::borrow::Cow<'_, str>>,
                                                        fn(&str, &std::net::TcpStream),
                                                        &std::net::TcpStream,
                                                    ),
                                            ),
                                            (
                                                "say",
                                                (|obj, write, stream| write(&*format!("PRIVMSG {} :{}", obj["channel"], obj["data"]), stream)),
                                            ),
                                            (
                                                "reply",
                                                (|obj, write, stream| {
                                                    std::iter::once(format!("PRIVMSG {} :{}: {}", obj["channel"], obj["nick"], obj["data"]))
                                                        .map(|data| write(&*data, stream))
                                                        .last()
                                                        .unwrap_or_default()
                                                }),
                                            ),
                                        ]
                                        .into_iter()
                                        .collect(),
                                    )
                                })
                                .and_then(|(obj, write, funcs)| {
                                    <&[(
                                        &str,
                                        for<'s> fn(
                                            std::collections::HashMap<&'static str, std::borrow::Cow<'_, str>>,
                                            &'s std::net::TcpStream,
                                            for<'a, 'b> fn(&'a str, &'b std::net::TcpStream),
                                            &std::collections::HashMap<
                                                &'static str,
                                                fn(
                                                    std::collections::HashMap<&'static str, std::borrow::Cow<'_, str>>,
                                                    fn(&str, &std::net::TcpStream),
                                                    &std::net::TcpStream,
                                                ),
                                            >,
                                        ),
                                    )]>::into_iter(&[
                                        (
                                            "!hello",
                                            (|mut obj, stream, write, funcs| {
                                                obj.insert("data", format!("hello {}!", obj["nick"]).into())
                                                    .map(drop)
                                                    .or(Some(()))
                                                    .map(|_| funcs["say"](obj, write, stream))
                                                    .unwrap_or_default()
                                            }),
                                        ),
                                        (
                                            "!source",
                                            (|mut obj, stream, write, funcs| {
                                                obj.insert(
                                                    "data",
                                                    "you can view this at https://github.com/museun/diet-semicola/blob/main/src/main.rs".into(),
                                                )
                                                .map(drop)
                                                .or(Some(()))
                                                .map(|_| funcs["reply"](obj, write, stream))
                                                .unwrap_or_default()
                                            }),
                                        ),
                                        (
                                            "!project",
                                            (|mut obj, stream, write, funcs| {
                                                obj.insert("data", "consider using a semicolon here: `\x3b`".into())
                                                    .map(drop)
                                                    .or(Some(()))
                                                    .map(|_| funcs["reply"](obj, write, stream))
                                                    .unwrap_or_default()
                                            }),
                                        ),
                                    ])
                                    .flat_map(|(cmd, func)| {
                                        obj["input"]
                                            .split_once(' ')
                                            .map(|(head, _)| head)
                                            .or(Some(&*obj["input"]))
                                            .filter(|head| head == cmd)
                                            .map(|_| func(obj.clone(), stream, write, &funcs))
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
