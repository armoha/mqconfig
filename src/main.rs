use indexmap::IndexMap;
use itertools::Itertools;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use toml::Value;

fn main() {
    let path = Path::new("config.toml");
    let display = path.display();
    let mut file = match File::open(&path) {
        Err(why) => panic!("파일 {} 을 여는데 실패했습니다: {}", display, why),
        Ok(file) => file,
    };
    let mut s = String::new();
    match file.read_to_string(&mut s) {
        Err(why) => panic!("파일 {} 을 읽는데 실패했습니다: {}", display, why),
        Ok(_) => (),
    }
    let value = match s.parse::<Value>() {
        Err(why) => panic!("파일 {} 작성 양식이 올바르지 않습니다: {}", display, why),
        Ok(value) => value,
    };
    // let value = value.as_table();
    let config = match Config::try_new(value) {
        Err(why) => panic!("파일 {} 작성 양식이 올바르지 않습니다: {}", display, why),
        Ok(config) => config,
    };
    config.export();
}

#[derive(Default)]
struct Config {
    hints1: Vec<String>,
    hints2: Vec<String>,
    lengths: Vec<u32>,
    display_answers: Vec<String>,
    answer_counts: Vec<u8>,
    chat_events: IndexMap<String, u32>,
    answers: Vec<Vec<u32>>,
    id_to_answers: IndexMap<u32, String>,
}
impl Config {
    fn export(&self) {
        let path = Path::new("musicConfig.eps");
        let display = path.display();
        let mut file = match File::create(&path) {
            Err(why) => panic!("파일 {} 을 만드는데 실패했습니다: {}", display, why),
            Ok(file) => file,
        };
        let output = format!(
            r#"/*
[chatEvent]
__addr__: 0x58D900
{}*/
const A = py_eval("lambda *args: EUDArray(args)");
const MusicNumber = {};  //음악갯수
const MusicHint1 = A(Db("{}"));
const MusicHint2 = A(Db("{}"));
const MusicLength = A({});
const MusicAnswer = A(Db("{}"));
const answerCount = A({});  // 맞춰야 넘어가는 개수
const answerLen = A({});  // 문제별 정답 총 개수
const answers = A(A({}));  // 문제별 채팅 인식 목록
const answerText = A(A(Db("{}")));  // 문제별 실제 정답 텍스트"#,
            self.chat_events
                .iter()
                .map(|(k, v)| format!("{} : {}\n", k, v))
                .collect::<String>(),
            self.answers.len(),
            self.hints1.join(r#""), Db(""#),
            self.hints2.join(r#""), Db(""#),
            self.lengths.iter().join(", "),
            self.display_answers.join(r#""), Db(""#),
            self.answer_counts.iter().join(", "),
            self.answers.iter().map(|x| x.len()).join(", "),
            self.answers
                .iter()
                .map(|x| x.iter().join(", "))
                .join("), A("),
            self.answers
                .iter()
                .map(|x| x
                    .iter()
                    .map(|y| self.id_to_answers[y].clone())
                    .join(r#""), Db(""#))
                .join(r#"")), A(Db(""#)
        );
        match file.write_all(output.as_bytes()) {
            Err(why) => panic!("파일 {} 에 작성하는데 실패했습니다: {}", display, why),
            Ok(_) => (),
        };
        println!("파일 {} 을 성공적으로 생성했습니다!", display);
    }

    fn try_new(toml: Value) -> Result<Config, String> {
        let mut config = Config::default();
        for i in 1..=7 {
            config.chat_events.insert(format!("!강퇴{}", i), 999 + i);
        }
        let value = toml.as_table().ok_or("전체 구조 이상")?;
        for (title, table) in value {
            let quiz = Quiz::try_new(table).map_err(|s| format!("곡 {} 오류: {}", title, s))?;
            config.hints1.push(quiz.hints[0].clone());
            config.hints2.push(quiz.hints[1].clone());
            config.lengths.push(quiz.length);
            config.display_answers.push(title.to_owned());
            config.answer_counts.push(quiz.answer_count);
            let mut answers: Vec<u32> = Vec::new();
            for a in quiz.answers {
                match config.chat_events.get(&a) {
                    None => {
                        let mut i = config.chat_events.len() as u32 - 5;
                        if i >= 1000 {
                            i += 7;
                        }
                        answers.push(i);
                        config.chat_events.insert(a.clone(), i);
                        config.id_to_answers.insert(i, a);
                    }
                    Some(i) => answers.push(*i),
                };
            }
            config.answers.push(answers);
        }
        Ok(config)
    }
}

struct Quiz {
    hints: Vec<String>,
    length: u32,
    answers: Vec<String>,
    answer_count: u8, // NonZero
}
impl Quiz {
    fn try_new(table: &Value) -> Result<Self, String> {
        let mut quiz = Quiz {
            hints: Vec::new(),
            length: 0,
            answers: Vec::new(),
            answer_count: 1,
        };
        let table = table
            .as_table()
            .ok_or(format!("{} 는 테이블이어야합니다", table))?;
        for (k, v) in table {
            match k.as_str() {
                "힌트" => {
                    for s in v
                        .as_array()
                        .ok_or(format!("힌트 목록 {} 은 배열이어야합니다.", v))?
                    {
                        quiz.hints.push(
                            s.as_str()
                                .ok_or(format!("힌트 {} 는 문자열이어야합니다.", s))?
                                .to_owned(),
                        );
                    }
                }
                "길이" => {
                    quiz.length = v
                        .as_integer()
                        .ok_or(format!("길이 {} 는 숫자여야합니다.", v))?
                        .try_into()
                        .map_err(|x| format!("{} : 길이 {} 는 양수여야합니다.", x, v))?
                }
                "정답" => {
                    for s in v
                        .as_array()
                        .ok_or(format!("정답 목록 {} 은 배열이어야합니다.", v))?
                    {
                        quiz.answers.push(
                            s.as_str()
                                .ok_or(format!("정답 {} 은 문자열이어야합니다.", s))?
                                .to_owned(),
                        );
                    }
                }
                "답개수" => {
                    quiz.answer_count = v
                        .as_integer()
                        .ok_or(format!("답개수 {} 는 숫자여야합니다.", v))?
                        .try_into()
                        .map_err(|x| format!("{} : 답개수 {} 는 1부터 192까지여야합니다.", x, v))?
                }
                s => panic!(
                    r#"{} 는 유효한 설정이 아닙니다.
                    설정 목록: "힌트", "길이", "정답", "답개수"
                    주석을 넣고 싶다면 앞에 # 을 쓰세요"#,
                    s
                ),
            };
        }
        if quiz.hints.len() != 2 {
            Err("힌트는 2개여야합니다.".to_owned())
        } else if quiz.answer_count == 0 || quiz.answer_count > 192 {
            Err("답개수는 1부터 192까지여야합니다.".to_owned())
        } else if quiz.answers.len() < quiz.answer_count as usize {
            Err("정답이 필요한 답개수보다 적습니다.".to_owned())
        } else {
            Ok(quiz)
        }
    }
}
