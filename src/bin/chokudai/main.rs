use rand::Rng;

struct Moves {
    label: Vec<usize>,
    door: Vec<usize>,
}

fn main() {
    //最初にフィールドサイズを受け取る (TODO: libに投げる)
    let n = 6;
    let q = n * 18;
    let mut m = Moves {
        label: vec![],
        door: vec![],
    };
    let mut rnd = rand::rng();

    //"0"~"5"の長さqのランダムな文字列Sを生成
    let mut s = String::new();
    for _ in 0..q {
        let c: usize = rnd.random_range(0..6);
        s.push_str(&c.to_string());
        m.door.push(c);
    }
    //以下の形式でsを出力する
    //./run post explore '{"plans":["315110321151051010104433505402153441105252040120553433"]}'
    println!("./run post explore '{{\"plans\":[\"{}\"]}}'", s);
    //eprintln!("{:?}", s);
    let s = s;
    //Sを投げて結果を受け取る
    let r = query(s);
    for c in r.chars() {
        m.label.push(c.to_digit(10).unwrap() as usize);
    }

    //推測を行う
    //4グループの個数を適当に分ける
    let mut nums = vec![0, 0, 0, 0];
    let mut cnts = vec![0, 0, 0, 0];
    for i in 0..m.door.len() {
        //eprintln!("label: {}, door: {}", m.label[i], m.door[i]);
        cnts[m.label[i]] += 1;
    }
    //頻度ごとに割り当てる
    for _ in 0..n {
        let mut best = 0.0;
        let mut id = 0;
        for i in 0..4 {
            let p = (cnts[i] as f64 + 0.01) / (nums[i] as f64 + 0.01);
            if p > best {
                best = p;
                id = i;
            }
        }
        nums[id] += 1;
    }
    //roomの数を出力
    eprintln!("nums: {:?}", nums);

    let mut label_start = vec![];
    label_start.push(0);
    for i in 0..3 {
        label_start.push(label_start[i] + nums[i]);
    }

    loop {
        //ランダムにlabelを割り当てる
        let mut ans = vec![0; m.label.len()];
        let mut rng = rand::rng();
        for i in 0..m.door.len() {
            ans[i] = label_start[m.label[i]] + rng.random_range(0..nums[m.label[i]]);
        }

        let mut loop_cnt = 0;
        let mut to = vec![vec![0; 6]; n];
        let mut wrong = error_check(&ans, &m, n);
        loop {
            loop_cnt += 1;
            if loop_cnt % 10000 == 0 {
                //eprintln!("loop_cnt: {}, wrong: {}", loop_cnt, wrong.0);
            }
            let ans_change = rnd.random_range(0..m.label.len());
            let mut new_ans = ans.clone();
            new_ans[ans_change] =
                label_start[m.label[ans_change]] + rnd.random_range(0..nums[m.label[ans_change]]);
            let (new_wrong, new_to) = error_check(&new_ans, &m, n);
            if new_wrong <= wrong.0 {
                if new_wrong < wrong.0 {
                    //println!("loop_cnt: {}, wrong: {}", loop_cnt, new_wrong);
                }
                wrong = (new_wrong, new_to.clone());
                ans = new_ans.clone();
                to = new_to.clone();
            }

            if wrong.0 == 0 {
                break;
            }
            if loop_cnt >= 20000 {
                break;
            }
        }
        if wrong.0 != 0 {
            eprintln!("error count: {}", wrong.0);
            continue;
        }

        //toからドアの対応を決める
        let ng = 9999;
        let mut to_door = vec![vec![ng; 6]; n];
        let mut ok = true;
        for i in 0..n {
            for j in 0..6 {
                //割り当て済みであればスキップ
                if to_door[i][j] != ng {
                    continue;
                }
                //to[i][j]から帰ってくるドアを探す
                let mut found = false;
                for k in 0..6 {
                    if to[to[i][j]][k] == i && to_door[to[i][j]][k] == ng {
                        to_door[i][j] = k;
                        to_door[to[i][j]][k] = j;
                        found = true;
                        break;
                    }
                }
                if !found {
                    ok = false;
                }
            }
        }

        if !ok {
            eprintln!("to_doorの割り当てに失敗しました");
            continue;
        }

        //toをこんな感じの出力にする
        //./run post guess '{"map":{"rooms":[0,1,2],"startingRoom":0,"connections":[{"from":{"room":0,"door":1},"to":{"room":1,"door":3}}]}}'
        let mut output = String::new();

        output.push_str("./run post guess '");
        output.push_str("{\"map\":{\"rooms\":[");
        let mut room_label_num = 0;
        for i in 0..n {
            while room_label_num < 3 && label_start[room_label_num + 1] <= i {
                room_label_num += 1;
            }
            output.push_str(room_label_num.to_string().as_str());
            if i != n - 1 {
                output.push_str(",");
            }
        }
        output.push_str(&format!("],\"startingRoom\":{},\"connections\":[", ans[0]));
        let mut connections = Vec::new();
        for i in 0..n {
            for j in 0..6 {
                if i > to[i][j] || i == to[i][j] && j > to_door[i][j] {
                    continue;
                }
                connections.push(format!(
                    "{{\"from\":{{\"room\":{},\"door\":{}}},\"to\":{{\"room\":{},\"door\":{}}}}}",
                    i, j, to[i][j], to_door[i][j]
                ));
            }
        }
        output.push_str(&connections.join(","));
        output.push_str("]}}'");
        eprintln!("{}", output);
        break;
    }
}

fn error_check(ans: &Vec<usize>, m: &Moves, n: usize) -> (usize, Vec<Vec<usize>>) {
    let mut to = vec![vec![0; 6]; n];
    //to_cnt[i][j][k]: 部屋iからラベルjのドアを通ったときに部屋kに行く回数
    let mut to_cnt = vec![vec![vec![0; n]; 6]; n];
    // ansからtoを多数決で予測する
    for i in 0..m.door.len() {
        to_cnt[ans[i]][m.door[i]][ans[i + 1]] += 1;
    }
    for i in 0..n {
        for j in 0..6 {
            let mut best = 0;
            let mut id = 0;
            for k in 0..n {
                if to_cnt[i][j][k] > best {
                    best = to_cnt[i][j][k];
                    id = k;
                }
            }
            if best == 0 {
                //0回だったらランダムに割り当てる
                id = rand::rng().random_range(0..n);
            }
            to[i][j] = id;
        }
    }
    //間違っている数を調べる
    let mut wrong = 0;
    for i in 0..m.door.len() {
        if to[ans[i]][m.door[i]] != ans[i + 1] {
            wrong += 1;
        }
    }
    (wrong, to)
}

//部屋を移動して結果を受け取る
fn query(s: String) -> String {
    //標準入力から答えを受け取る
    use std::io::{self, Write};
    let mut stdout = io::stdout();
    writeln!(stdout, "{}", s).unwrap();
    stdout.flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let input = input.trim();
    input.to_string()
}
