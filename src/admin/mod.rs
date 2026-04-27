pub mod metric;



pub fn hnts_shell_exec(input: &str, snpashot: String) -> String {
    let parts: Vec<&str> = input.splitn(2, ' ').collect();
    let cmd  = parts[0];
    let args = parts.get(1).copied().unwrap_or("").trim();

    match cmd {
"help" => "доступные команды:#NL#
#T#help          — эта справка#NL#
#T#status        — статус сервера#NL#
#T#version       — версия сервера#NL#
#T#echo <текст>  — вернуть текст#NL#
#T#news set <id> — привязать пост к сайдбару#NL#
#T#clear         — очистить экран\
".into(),

"status" => "сервер работает нормально".into(),

"version" => concat!("HentaiTracker v", env!("CARGO_PKG_VERSION")).into(),

"echo" => {
    if args.is_empty() { String::new() } else { args.to_string() }
}

"fetch" => format!("\
░░░█░█░█▀▀░█▀█░▀█▀░█▀█░▀█▀░░░░#C#purple#C##NL#\
░░░█▀█░█▀▀░█░█░░█░░█▀█░░█░░░░░#C#purple#C##NL#\
░░░▀░▀░▀▀▀░▀░▀░░▀░░▀░▀░▀▀▀░░░░#C#purple#C##NL#\
░▀█▀░█▀▄░█▀█░█▀▀░█░█░█▀▀░█▀▄░░#C#cyan#C##NL#\
░░█░░█▀▄░█▀█░█░░░█▀▄░█▀▀░█▀▄░░#C#cyan#C##NL#\
░░▀░░▀░▀░▀░▀░▀▀▀░▀░▀░▀▀▀░▀░▀░░#C#cyan#C##NL#\
{}", snpashot).into(),

"clear" => "\x1b[2J".into(),

"news" => {
    let sub_parts: Vec<&str> = args.splitn(2, ' ').collect();
    match sub_parts[0] {
        "set" => {
            let id_str = sub_parts.get(1).unwrap_or(&"").trim();
            match id_str.parse::<i32>() {
                Ok(id)  => format!("news:set:{}", id),
                Err(_)  => "ошибка: укажи числовой id поста. пример: news set 42".into(),
            }
        }
        _ => "доступные подкоманды:#NL##T#news set <post_id>".into(),
    }
},

_ => format!("неизвестная команда: {cmd}. введите help для справки"),
    }
}
