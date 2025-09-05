#![allow(clippy::collapsible_if, clippy::cast_abs_to_unsigned, clippy::ptr_arg)]
use clap::Parser;
use icfpc2025::judge::*;
use rand::prelude::*;

struct Moves {
    label: Vec<usize>,
    door: Vec<usize>,
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(
        short = 'i',
        long = "input",
        help = "Read input from file instead of stdin"
    )]
    input: Option<String>,
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

    let mut label_id = vec![0; n];
    for i in 0..4 {
        for j in 0..nums[i] {
            label_id[label_start[i] + j] = i;
        }
    }

    loop {
        //ランダムにlabelを割り当てる
        let mut ans = vec![0; m.label.len()];
        let mut rng = rand::rng();
        for i in 0..m.door.len() {
            ans[i] = label_start[m.label[i]] + rng.random_range(0..nums[m.label[i]]);
        }

        let mut loop_cnt = 0;
        let mut wrong = error_check(&ans, &m, n);
        let mut to = wrong.1.clone();
        let mut best_wrong = wrong.0;
        let mut not_update = 0;
        let mut best_ans = ans.clone();
        let mut best_to = to.clone();

        loop {
            loop_cnt += 1;
            not_update += 1;

            if loop_cnt % 10000 == 0 {
                //eprintln!("loop_cnt: {}, wrong: {}", loop_cnt, wrong.0);
            }

            let mut new_ans = vec![];
            let rn = rnd.random_range(0..10);
            if rn <= 9 {
                let ans_change = rnd.random_range(0..m.label.len());
                let mut has_error = false;
                if ans_change != m.label.len() - 1 {
                    if to[ans[ans_change]][m.door[ans_change]] != ans[ans_change + 1] {
                        has_error = true;
                    }
                }
                if ans_change != 0 {
                    if to[ans[ans_change - 1]][m.door[ans_change - 1]] != ans[ans_change] {
                        has_error = true;
                    }
                }
                if ans_change != m.label.len() - 1 {
                    if to[ans[ans_change]][m.door[ans_change]] != ans[ans_change + 1] {
                        has_error = true;
                    }
                }

                if !has_error {
                    continue;
                }
                new_ans = ans.clone();
                new_ans[ans_change] = label_start[m.label[ans_change]]
                    + rnd.random_range(0..nums[m.label[ans_change]]);
            } else if rn < 10 {
                //シャッフル法2: to[i][j]をランダムに決め打ちし、ansにそれを全部の部屋に対して反映させる
                //i,jはランダムに選び、to[i][j]は、現在のto[i][j]とlabelが同じな中からランダムで選ぶ
                let i = rnd.random_range(0..n);
                let j = rnd.random_range(0..6);
                let now_label = label_id[to[i][j]];
                to[i][j] = label_start[now_label] + rnd.random_range(0..nums[now_label]);
                new_ans = ans.clone();
                for k in 0..m.door.len() {
                    if new_ans[k] == i {
                        new_ans[k + 1] = to[i][j];
                    }
                }
            }

            let (new_wrong, new_to) = error_check(&new_ans, &m, n);
            if new_wrong <= wrong.0 {
                if new_wrong < best_wrong {
                    println!("loop_cnt: {}, wrong: {}", loop_cnt, new_wrong);
                    best_wrong = new_wrong;
                    best_ans = new_ans.clone();
                    best_to = new_to.clone();
                    not_update = 0;
                    //println!("loop_cnt: {}, wrong: {}", loop_cnt, new_wrong);
                }
                wrong = (new_wrong, new_to.clone());
                ans = new_ans.clone();
                to = new_to.clone();
            }

            if wrong.0 == 0 {
                break;
            }

            if not_update >= 20000 {
                //toだけで上手く行くか一応チェックする
                let (wrong2, new_ans) = to_check(&ans, &label_id, &to, &m);
                if wrong2 == 0 {
                    eprintln!("find to_check");
                    ans = new_ans;
                    to = wrong.1.clone();
                    wrong.0 = 0;
                    break;
                } else {
                    //eprint!("to_check wrong: {}\n", wrong2);
                }

                //いったん最強解に戻す
                ans = best_ans.clone();
                to = best_to.clone();

                let r = rnd.random_range(0..2);
                if r == 0 {
                    let label_change = rnd.random_range(0..4);
                    //シャッフル法1: 特定のlabelをランダムに選んで全部ランダム化する
                    for i in 0..m.door.len() {
                        if m.label[i] == label_change {
                            ans[i] =
                                label_start[m.label[i]] + rnd.random_range(0..nums[m.label[i]]);
                        }
                    }
                } else if r == 1 {
                    //シャッフル法2: to[i][j]をランダムに決め打ちし、ansにそれを全部の部屋に対して反映させる
                    //i,jはランダムに選び、to[i][j]は、現在のto[i][j]とlabelが同じな中からランダムで選ぶ
                    let i = rnd.random_range(0..n);
                    let j = rnd.random_range(0..6);
                    let now_label = label_id[to[i][j]];
                    to[i][j] = label_start[now_label] + rnd.random_range(0..nums[now_label]);
                    for k in 0..m.door.len() {
                        if ans[k] == i {
                            ans[k + 1] = to[i][j];
                        }
                    }
                }

                let res = error_check(&ans, &m, n);
                wrong = res;
                to = wrong.1.clone();
                not_update = 0;
            }

            if loop_cnt >= 50000000 {
                break;
            }
        }
        if wrong.0 != 0 {
            eprintln!("error count: {}", best_wrong);
            continue;
        }

        //toからドアの対応を決める
        let ng = 9999;
        let mut to_door = vec![vec![ng; 6]; n];
        let mut found = false;
        for _ in 0..1000 {
            //toの割り当て直しからする
            let ret = error_check(&ans, &m, n);
            to = ret.1;
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
                continue;
            } else {
                found = true;
                break;
            }
        }
        if !found {
            eprintln!("not found to_door");
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
            wrong += 10001;
        }
    }

    //ドアの個数の不整合チェック
    let mut door_cnt = vec![vec![0; n]; n];
    for i in 0..n {
        for j in 0..6 {
            door_cnt[i][to[i][j]] += 1;
        }
    }

    for i in 0..n {
        for j in 0..n {
            if door_cnt[i][j] != door_cnt[j][i] {
                wrong += 70100 * (door_cnt[i][j] as isize - door_cnt[j][i] as isize).abs() as usize;
            }
        }
    }

    (wrong, to)
}

//toを使ってansを作ってみた時に上手く行くかチェックする
fn to_check(
    ans: &[usize],
    label_id: &[usize],
    to: &Vec<Vec<usize>>,
    m: &Moves,
) -> (usize, Vec<usize>) {
    let mut ret_ans = vec![0; m.label.len()];
    let mut now = ans[0];
    let mut wrong = 0;
    ret_ans[0] = now;
    for i in 0..m.door.len() {
        let next = to[now][m.door[i]];
        if label_id[next] != m.label[i + 1] {
            wrong += 1;
        }
        now = next;
        ret_ans[i + 1] = now;
    }
    (wrong, ret_ans)
}
