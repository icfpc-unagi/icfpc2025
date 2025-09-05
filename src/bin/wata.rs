use icfpc2025::judge::*;
use rand::prelude::*;

struct Moves {
    label: Vec<usize>,
    door: Vec<usize>,
}

fn main() {
    let mut judge = get_judge_from_stdin();
    let n = judge.num_rooms();
    let q = n * 18;
    let mut m = Moves {
        label: vec![],
        door: vec![],
    };
    let mut rnd = rand::rng();

    //"0"~"5"の長さqのランダムな文字列Sを生成
    let mut plan = vec![];
    for _ in 0..q {
        let c: usize = rnd.random_range(0..6);
        plan.push(c);
        m.door.push(c);
    }
    let plans = vec![plan];
    let r = judge.explore(&plans);
    m.label = r[0].clone();

    //推測を行う
    //4グループの個数を適当に分ける
    let mut nums = vec![0, 0, 0, 0];
    let mut cnts = [0usize; 4];
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

        let mut out = Guess {
            rooms: vec![0; n],
            start: ans[0],
            graph: vec![[(0, 0); 6]; n],
        };
        let mut room_label_num = 0;
        for i in 0..n {
            while room_label_num < 3 && label_start[room_label_num + 1] <= i {
                room_label_num += 1;
            }
            out.rooms[i] = room_label_num;
        }
        for i in 0..n {
            for j in 0..6 {
                out.graph[i][j] = (to[i][j], to_door[i][j]);
            }
        }
        judge.guess(&out);
        break;
    }
}

fn error_check(ans: &[usize], m: &Moves, n: usize) -> (usize, Vec<Vec<usize>>) {
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
