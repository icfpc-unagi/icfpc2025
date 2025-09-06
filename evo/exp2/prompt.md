# 概要

我々は今「ICFP Programming Contest 2025」というプログラミングコンテストに参加しています。あなたはこの問題に関する試行錯誤を手伝って下さい。

以下では、まず最初にICFP Programming Contest 2025の問題文を原文ママで提示します。

次に、その取り組みの一部として、今からあなたにやってほしいことを説明します。


# ICFP Programming Contest 2025

## TASK DESCRIPTION

Contestants are tasked with constructing a map of a labyrinthine library by collecting information during repeated exploratory expeditions. *This is version 1.2 — see the changelog on the task page.*

## BACKGROUND

In which our characters, Adso & William, encounter the library at the Ædificium, and seek to uncover the secrets therein.

Adso of Milner and his master, William of Backus, have at last arrived at the famed monastery of St. Kleene, under the stewardship of the monks of the Holy Order of Evaluation, home of the world-renowned Library of Lambdas, where it is said that all of the world’s functions may be found.

The library makes up the greatest part of the Ædificium: the large, imposing, fortress-like building casting a long shadow over the cloisters in the setting sun. It is closely guarded by the Abbot and his librarian, Alonzo of Curry, so William and Adso can only gain access to the library via skulduggery, sneaking into the Ædificium when the monks and nuns are busy at the International Conference on Functional Programming.

The Ædificium is huge, and only Alonzo and the Abbot know all of its chambers and passages. Some even say that a curse lies on the library, and that any who enter unbidden will never find their way out again. William, a learned scholar of great intuition and logic, dismisses such superstitions out of hand, steadfast in his determination to uncover the secrets of the library. He exclaims in the fashion of his master, Hilbert, “*Wir müssen wissen, wir werden wissen!*”

Adso, ever doubtful and unsure, inquires of his master, “How shall we know where to go without the guidance of the librarian? We do not even have a map!”

William, a devious glint behind his eyes, replies, “Well, my dear Adso, we shall have to make one.”

## THE ÆDIFICIUM

In which the structure of the Library is described, as well as the nature of the task before William & Adso.

The Ædificium is comprised of a number of hexagonal rooms, each side of which has a door, labelled with a number **0–5**. Through each door lies a passage to another room. The rooms themselves are also labelled, but the label is in a language that Adso cannot read. William understands the language, but with his failing eyesight, he can only discern the first **two bits** of the label. More than one room may have the same label. Passages may lead to the same room from which they started — or even the same door.

To avoid getting lost, Adso & William must devise a **route plan** before entering the library. A route plan consists of a series of numbers **0–5**, indicating the sequence of doors through which they plan to travel, starting from the initial room. The route plans always start from the same room.

As they travel through the library according to their route plan, Adso will record the **2-bit integers** that William reads from the label of each room. Thus, after executing a route plan of length *x*, Adso will have a record consisting of *x + 1* 2-bit integers.

The task before Adso & William is to construct a series of route plans such that they gain the information necessary to construct an accurate **map of the library**, in the form of an undirected graph, using as few expeditions into the library as possible.

## PROTOCOL

In which the format of route plans and Adso’s records are specified, as well as the representation of the map.

**Base URL**

```
https://31pwr5t6ij.execute-api.eu-west-2.amazonaws.com/
```

### `POST /register` — registers a new team

**Request Body**

```json
{
  "name": "string",
  "pl": "string",
  "email": "string"
}
```

**Response Body**

```json
{
  "id": "string"
}
```

The `id` given in the response is secret and used to identify your team in future requests. Remember it but do not disclose it publicly!

---

### `POST /select` — selects a problem to solve

**Request Body**

```json
{
  "id": "string",
  "problemName": "string"
}
```

**Response Body**

```json
{
  "problemName": "string"
}
```

* The `id` should be exactly the same string produced by `/register`.
* The `problemName` may be any of the available problems, the full list of which is available on the leaderboard page. Each problem has a specified number of rooms, but the exact layout of the map is randomly generated each time a problem is selected.
* For testing purposes, a simple three-room labyrinth, small enough to be solvable by hand, may also be selected by using the problem name **`"probatio"`**.
* **Note:** if a problem is already selected, POSTing to `/select` will discard the old problem and generate a new one to solve.

---

### `POST /explore` — explores the ædificium

**Request Body**

```json
{
  "id": "string",
  "plans": ["string"]
}
```

**Response Body**

```json
{
  "results": [[0]],
  "queryCount": 0
}
```

* Only POST to `/explore` after selecting a problem with `/select`.
* The `id` should be exactly the same string produced by `/register`.
* The `plans` field should consist of a list of route plans, each represented as a **string of digits 0–5**, specifying the numbers of each door to enter.
  *Example:* `"0325"` enters door 0, then 3, then 2, then 5.
* **Route length limit:** they can enter at most **`18n` doorways per night**, where *n* is the number of rooms in the library.
* The `results` field consists of a list of Adso’s records, one for each route plan submitted. Each record is a list of integer values — the 2-bit integer values observed by William upon entering each room.
* The `queryCount` field contains the total number of expeditions made into the Ædificium so far.
* **Batching incentive:** multiple route plans may be submitted in a single HTTP request; to incentivise batching, an additional **one-point `queryCount` penalty** applies per `/explore` request made.

---

### `POST /guess` — submit a candidate map

**Request Body**

```json
{
  "id": "string",
  "map": {
    "rooms": [0],
    "startingRoom": 0,
    "connections": [
      {
        "from": { "room": 0, "door": 0 },
        "to":   { "room": 0, "door": 0 }
      }
    ]
  }
}
```

**Response Body**

```json
{
  "correct": true
}
```

* Only POST to `/guess` after selecting a problem with `/select`.
* The `map` field describes the layout:

  * **`rooms`**: list of the 2-bit integer labels read by William, one per room. Rooms are identified by their index in this list.
  * **`startingRoom`**: the index of the initial room.
  * **`connections`**: list of objects specifying how each room is connected. Each side specifies a `{ room, door }` pair where `door` is **0–5**.
* The graph constructed is **undirected**. If you connect, e.g., from door 3 of room 5 to door 0 of room 2, there is no need to add the reverse connection — it already exists.
* The field `correct` is **true iff** the submitted map is equivalent to the map generated when `/select` was invoked. By *equivalent*, we mean they have the same number of rooms and are indistinguishable by any route plan.
* If a correct map is submitted and your `queryCount` improves your previous score for the currently selected problem, your score is updated.
* **After `/guess`** the problem is **deselected** and the library cleared. If your guess was incorrect, you must start again with `/select`.

## SCORING

In which the criteria by which entries are evaluated are given, along with pointers to global scoreboards.

* Grading of correct maps is based on the **number of expeditions** required to produce the graph.
* Each team is ranked on the **local leaderboard** for each problem by fewest expeditions (ties among teams without a correct solution).
* The **global leaderboard** uses the **Borda count** (as last year):
  For each problem, each team gets one point for every other team that places strictly worse. The team with the most points across all problems ranks first.
  *Note:* absolute scores for a problem do not matter — only the relative order.
* Leaderboards update every few minutes through most of the contest, but are **temporarily frozen** two hours before the end of the lightning round and in the last two hours of the contest.

## SUBMISSION

In which the method of participation and entry submission is described, as well as the dates during which this is available.

To be considered for prizes, your team must **submit your code** via the following Google Form. Include your `id` from `/register` and a URL where your code can be accessed (e.g., a Git repository). Don’t make your code public until after the contest.

This form **closes 3 hours after the end of the contest**. It is not necessary to make a separate submission for the lightning round, but please include a README indicating which parts are from the first 24 hours.

**Note:** We also hand out a “jury prize”, so topping the scoreboard is not required to win (though it certainly helps!).



# あなたにやって欲しいこと

今からあなたがやるのは **1ターンでやること** になるべく特化した、解答プログラムを作成・改善することです。あなたは `solve` 関数の中を実装・改善します。これは、`Judge` オブジェクトを受け取り、そのオブジェクトを通じて問題に関するやり取りを行います。`Judge` オブジェクトの各関数は、上の問題概要における各エンドポイントに対応しています。

```rust
pub struct Guess {
    pub rooms: Vec<usize>,
    pub start: usize,
    pub graph: Vec<[(usize, usize); 6]>,
}

pub trait Judge {
    fn num_rooms(&self) -> usize;
    fn problem_name(&self) -> &str;
    fn explore(&mut self, plans: &[Vec<usize>]) -> Vec<Vec<usize>>;
    fn guess(&self, out: &Guess) -> bool;
}
```

`num_room` は6, 12, 18, 24, 30のケースがあります。順位表から得られる他チームの情報から、全てのケースについて **1ターンで完了することができる**、すなわちexploreの呼び出しが1回で解く事ができることがわかっています。この問題のルールをよく読めば分かる通り、実際には何回も挑戦できるので、毎回安定して1回で解ける必要はありません。ただ、exploreを1回だけ呼び出し、そこで得られた情報から次にはもうguessをする、そういった解法が求められています。

あなたがこれから作るプログラムは、私が作成したテストケースで評価されます。 `num_room` が18のテストケースを複数回試し、「正しい解答を出すまでの実行時間の中間値」を評価します（正しくなかったものや実行が終了しなかったものについては実行時間制限の長さをそのまま実行時間とみなします）。あなたは **この評価値を最小化** するためにプログラムを作成・改善して下さい。

（あなたへのフィードバックで与えられる `combined_score` は上述の「正しい解答を出すまでの実行時間の中間値」にマイナスを付けたものです。これを最大化するようにして下さい。）

## Tips

* plansを短くすることは、結果を悪化させます。plansの長さは18*num_roomsを保って下さい。正しい推測を行うために、これだけの長さの情報が必要です。短くすると、推測に失敗してしまいます。
* 実際の実行時間はSATソルバーが大部分を占めます。例えば、SATソルバーが問題を解きやすくすることが良いでしょう。例えば、ヒントになるようなclauseを追加する、筋の良い条件分岐のための良い変数を追加する、などでしょうか。
