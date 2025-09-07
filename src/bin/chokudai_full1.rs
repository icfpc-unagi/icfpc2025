#![cfg_attr(feature = "skip_lint", allow(clippy::all, clippy::pedantic, warnings))]
use icfpc2025::judge::*;
use rand::Rng;

fn check_samenum(a: usize, n: usize) -> bool {
    for i in 1..n {
        if a % 4 != (a >> (2 * i)) % 4 {
            return true;
        }
    }
    false
}

fn main() {
    let mut judge = get_judge_from_stdin_with(false);
    let n = judge.num_rooms();
    let q_limit = n * 6;

    let mut steps = vec![];

    let mut rnd = rand::rng();

    let iwi_vec = icfpc2025::routes::get_plan(n);

    let mut qnum = 2;
    let mut q_perf = 12;
    while q_perf < q_limit {
        qnum += 1;
        q_perf = (1 << (2 * qnum)) - 4;
    }
    for i in 0..qnum {
        steps.push(vec![]);
    }

    // i -> id
    let mut itoid = vec![0; 1 << (2 * qnum)];

    // assign id
    let mut now_id = 0;
    for i in 0..q_limit {
        steps[0].push((None, iwi_vec[i]));
        while !check_samenum(now_id, qnum - 1) {
            now_id += 1;
        }
        for k in 1..qnum {
            steps[k].push((Some((now_id >> (2 * (k - 1))) % 4), iwi_vec[i]));
        }
        itoid[now_id] = i;
        now_id += 1;
    }

    let mut vec_id = vec![];
    let mut vec_leader = vec![];
    let mut vec_label = vec![];
    let mut vec_nums = 0;

    let r = judge.explore(&steps);

    for i in 0..q_limit + 1 {
        let mut num = 0;
        for k in 1..qnum {
            num += (r[k][i] as usize) << (2 * (k - 1));
        }
        if !check_samenum(num, qnum - 1) {
            eprintln!("new id: {}, label: {}, num {}", i, r[0][i], num);
            vec_id.push(vec_nums);
            vec_leader.push(i);
            vec_label.push(r[0][i]);
            vec_nums += 1;
        } else {
            vec_id.push(vec_id[itoid[num]]);
        }
    }

    eprint!("vec_nums: {} / {}, ", vec_nums, n);

    if vec_nums < n {
        panic!("first-phase failed");
    }

    let mut graph = vec![vec![!0; 6]; n];
    for i in 0..q_limit {
        let id1 = vec_id[i];
        let id2 = vec_id[i + 1];
        let door = iwi_vec[i];
        graph[id1][door] = id2;
    }

    let ret = false; //update_graph(&mut graph, n);

    while !ret {
        //second phase
        //warshall floyd
        let mut dist = vec![vec![!0; n]; n];
        let mut next = vec![vec![!0; n]; n];
        for i in 0..n {
            dist[i][i] = 0;
            for d in 0..6 {
                let j = graph[i][d];
                if j != !0 {
                    dist[i][j] = 1;
                    next[i][j] = j;
                }
            }
        }
        for k in 0..n {
            for i in 0..n {
                for j in 0..n {
                    if dist[i][k] != !0 && dist[k][j] != !0 {
                        let nd = dist[i][k] + dist[k][j];
                        if dist[i][j] == !0 || nd < dist[i][j] {
                            dist[i][j] = nd;
                            next[i][j] = next[i][k];
                        }
                    }
                }
            }
        }

        //greedyで全部の頂点を辿る
        let mut visited = vec![false; n];
        let mut path = vec![];
        let mut now = 0;
        visited[now] = true;
        let mut sum_dist = 0;
        path.push(now);

        while path.len() < n {
            let mut best = !0;
            let mut nid = !0;
            for i in 0..n {
                if visited[i] {
                    continue;
                }
                if dist[now][i] != !0 && (best == !0 || dist[now][i] < best) {
                    best = dist[now][i];
                    nid = i;
                }
            }
            if nid == !0 {
                break;
            }
            visited[nid] = true;
            path.push(nid);
            now = nid;
            sum_dist += best;
            eprintln!(
                "path len: {}, sum_dist: {}, now: {}",
                path.len(),
                sum_dist,
                now
            );
        }

        //eprintln!("path len: {}, sum_dist: {}", path.len(), sum_dist);

        let mut pos = vec![];
        pos.push(path[0]);
        let mut door2 = vec![];
        for i in 0..path.len() - 1 {
            let path_dist = dist[path[i]][path[i + 1]];
            eprintln!("from {} to {} dist {}", path[i], path[i + 1], path_dist);
            let mut now = path[i];
            for t in 0..path_dist {
                let next_node = next[now][path[i + 1]];
                eprintln!("next_node: {}", next_node);
                if next_node == !0 {
                    eprintln!(
                        "error: no path from {}({}) to {}({})",
                        path[i],
                        i,
                        path[i + 1],
                        i + 1
                    );
                    break;
                }
                // Find the door that leads to next_node
                let mut found = false;
                for d in 0..6 {
                    if graph[now][d] == next_node {
                        door2.push(d);
                        now = next_node;
                        pos.push(now);
                        found = true;
                        break;
                    }
                }
                if !found {
                    eprintln!("error: no door from {} to {}", now, next_node);
                    break;
                }
            }
        }

        eprintln!("door2 len: {}", door2.len());
        let dstart = door2.len();

        let mut qnum2 = 2;
        let mut q_perf = 16;
        while q_perf < q_limit {
            qnum2 += 1;
            q_perf = 1 << (2 * qnum2);
        }

        let mut step2 = vec![];
        for i in 0..qnum2 {
            step2.push(vec![]);
        }

        //pos[i]に塗りつぶしながら進む
        for i in 0..door2.len() {
            for k in 0..qnum2 {
                step2[k].push((Some((pos[i] >> (2 * k)) % 4), door2[i]));
            }
        }

        //set step with random door
        while door2.len() < q_limit {
            let next_door = rnd.random_range(0..6);
            door2.push(next_door);
            for k in 0..qnum2 {
                step2[k].push((None, next_door));
            }
        }

        let r2 = judge.explore(&step2);

        let mut pre_id = pos[dstart - 1];

        //graphの更新
        for i in dstart..q_limit {
            let mut id = 0;
            for k in 0..qnum2 {
                id += (r2[k][i + 1] as usize) << (2 * k);
            }

            let d = door2[i];
            graph[pre_id][d] = id;
            eprintln!("update: {} --{}--> {}", pre_id, d, id);
            pre_id = id;
        }

        let ret2 = update_graph(&mut graph, n);
        if ret2 {
            break;
        }
    }

    //graphを出力形式にする
    let mut out_graph = vec![[(0, 0); 6]; n];
    let mut used = vec![vec![false; 6]; n];

    for i in 0..n {
        for j in 0..6 {
            let ni = graph[i][j];
            if ni == !0 {
                eprintln!("error: graph[{}][{}] == !0", i, j);
                continue;
            }
            if used[i][j] {
                continue;
            }
            for nj in 0..6 {
                if graph[ni][nj] == i && !used[ni][nj] {
                    out_graph[i][j] = (ni, nj);
                    out_graph[ni][nj] = (i, j);
                    used[i][j] = true;
                    used[ni][nj] = true;
                    break;
                }
            }
        }
    }

    //提出パート
    let out = Guess {
        rooms: vec_label.clone(),
        start: 0,
        graph: out_graph.clone(),
    };
    judge.guess(&out);
}

fn update_graph(graph: &mut Vec<Vec<usize>>, n: usize) -> bool {
    let mut go_sum = vec![vec![0; n]; n];
    let mut ret = true;
    for i in 0..n {
        for d in 0..6 {
            let j = graph[i][d];
            if j != !0 {
                go_sum[i][j] += 1;
            }
        }
    }
    let mut mat_sum = vec![vec![0; n]; n];
    for i in 0..n {
        for j in 0..n {
            mat_sum[i][j] = go_sum[i][j];
            if go_sum[j][i] > go_sum[i][j] {
                mat_sum[i][j] = go_sum[j][i];
            }
        }
    }
    let mut arr_sum = vec![0; n];
    for i in 0..n {
        for j in 0..n {
            arr_sum[i] += mat_sum[i][j];
        }
    }

    for i in 0..n {
        if arr_sum[i] < 6 {
            eprintln!("error: arr_sum[{}] == {}", i, arr_sum[i]);
            ret = false;
            continue;
        }
        let mut cnt = 0;
        let mut a = 0;
        for j in 0..n {
            if mat_sum[i][j] > go_sum[i][j] {
                cnt += 1;
                a = j;
            }
        }
        if cnt >= 2 {
            eprintln!("error: cnt[{}] == {}", i, cnt);
            ret = false;
            continue;
        }
        if cnt == 1 {
            for d in 0..6 {
                if graph[i][d] == !0 {
                    graph[i][d] = a;
                    go_sum[i][a] += 1;
                }
            }
        }
    }
    ret
}
