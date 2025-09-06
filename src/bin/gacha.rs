use icfpc2025::{judge::*, *};

struct PlanInfo {
    n: usize,
    plan: Vec<usize>,
    labels: Vec<usize>,
    diff: Vec<Vec<bool>>,
}

fn compute_diff(plan: &[usize], labels: &[usize]) -> Vec<Vec<bool>> {
    let m = labels.len();
    let t = plan.len();
    let mut diff = mat![false; m; m];
    for i in (0..m).rev() {
        for j in (0..m).rev() {
            if labels[i] != labels[j]
                || (i < t && j < t && plan[i] == plan[j] && diff[i + 1][j + 1])
            {
                diff[i][j] = true;
            }
        }
    }
    for i in 0..m {
        diff[i][i] = false;
        for j in 0..i {
            let v = diff[i][j] || diff[j][i];
            diff[i][j] = v;
            diff[j][i] = v;
        }
    }
    diff
}

fn acquire_plan_and_labels(judge: &mut dyn icfpc2025::judge::Judge) -> PlanInfo {
    let n = judge.num_rooms();

    let plan_str = if n == 30 {
        "413022551403315200442351124530532105441250342013450431221500235401332245541100524301142035531432234405215311350240015425104334025123304521120534103522445503114230032542135431452051002415012340523321554433021451132041025533014400351220440115324321342250451305502315412031045522433500142534115203442231104350310540221435132441500553020145133425523012412033443004231254411023052214500135410325345320121545042102113352405514315340154304422003355523411201445331322002514054225105033004323315550112134421441352532400511024124025405311304432221235"
    } else if n == 24 {
        "053421124355003145223044132102540153351203445023114200554324125133051042215014033152443520411325530244002234511032054154230134552103501221433402532514310044152332500144551240530123153410521354220330420524115043021334514011522400543355322502431104320154423513402104531230554420011342541350314220511225053310324405552341300214450322545125330150043123141012421453202513005434045013322443102352331551412002403415510035111204255404452032"
    } else {
        panic!("Unsupported number of rooms: {}", n);
    };

    let plan = plan_str
        .chars()
        .map(|c| c.to_digit(10).unwrap() as usize)
        .collect::<Vec<_>>();

    let steps: Vec<(Option<usize>, usize)> = plan.iter().copied().map(|d| (None, d)).collect();
    let labels = judge.explore(&[steps])[0].clone();
    let m = labels.len();
    let t = plan.len();
    debug_assert_eq!(m, t + 1);
    let diff = compute_diff(&plan, &labels);
    PlanInfo {
        n,
        plan,
        labels,
        diff,
    }
}

fn main() {
    let mut list = vec![];
    let mut judge = get_judge_from_stdin_with(true);
    for _ in 0..100 {
        judge.restart();
        let info = acquire_plan_and_labels(judge.as_mut());
        let mut diff_count = 0;
        for i in 0..info.labels.len() {
            for j in 0..i {
                if info.diff[i][j] {
                    diff_count += 1;
                }
            }
        }
        eprintln!("diff_count = {}", diff_count);
        // list.push(diff_count);
        let mut aib = mat![false; 4; 6; 4];
        for k in 0..info.plan.len() {
            let a = info.labels[k];
            let i = info.plan[k];
            let b = info.labels[k + 1];
            aib[a][i][b] = true;
        }
        let mut cnt = 0;
        for a in aib.iter() {
            for i in a.iter() {
                for b in i.iter() {
                    if !*b {
                        cnt += 1;
                    }
                }
            }
        }
        eprintln!("aib_missing = {}", cnt);
        // list.push(cnt);
        let mut label_door = mat![0; 4; 6];
        for i in 0..info.plan.len() {
            let door = info.plan[i];
            let label = info.labels[i];
            label_door[label][door] += 1;
        }
        let mut sum = 0.0;
        let mut num = [0; 4];
        for i in 0..info.n {
            num[i % 4] += 1;
        }
        for i in 0..4 {
            for j in 0..6 {
                let expected = num[i] as f64 / info.n as f64 * info.plan.len() as f64 / 6.0;
                sum += (expected - label_door[i][j] as f64).powi(2);
            }
        }
        list.push(sum);
        eprintln!("label-door-chi2 = {}", sum);
    }
    list.sort_by(|a, b| b.partial_cmp(a).unwrap());
    dbg!(&list);
}
