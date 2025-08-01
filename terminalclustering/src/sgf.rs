use std::{fs, vec};

use sgf_parse::SgfProp;

pub fn load_sgf(path: &str) -> anyhow::Result<sgf_parse::GameTree> {
    let content = fs::read_to_string(path)?;
    let mut trees = sgf_parse::parse(&content)?;
    trees
        .pop()
        .ok_or_else(|| anyhow::Error::msg("no game tree found"))
}

fn move_to_string(mv: &sgf_parse::go::Move) -> String {
    let rank: &'static [u8] = "ABCDEFGHJKLMNOPQRST".as_bytes();
    match mv {
        sgf_parse::go::Move::Pass => "pass".into(),
        sgf_parse::go::Move::Move(point) => {
            (rank[usize::from(point.x)] as char).to_string() + (point.y + 1).to_string().as_str()
        }
    }
}

fn string_to_move(mv: &String) -> sgf_parse::go::Move {
    if mv == "pass" {
        return sgf_parse::go::Move::Pass;
    }
    let rank: &'static [u8] = "ABCDEFGHJKLMNOPQRST".as_bytes();
    let x_char = mv.as_bytes()[0];
    let x = rank.iter().position(|v| *v == x_char).unwrap() as u8;
    let y = mv[1..].parse::<u8>().unwrap() - 1;
    sgf_parse::go::Move::Move(sgf_parse::go::Point { x, y })
}

pub fn sgf_to_stones(sgf: &sgf_parse::GameTree) -> anyhow::Result<Vec<(String, String)>> {
    Ok(sgf
        .as_go_node()?
        .main_variation()
        .filter_map(|node| {
            let Some(prop) = node.get_move() else {
                return None;
            };
            match prop {
                sgf_parse::go::Prop::B(mv) => Some(("B".into(), move_to_string(mv))),
                sgf_parse::go::Prop::W(mv) => Some(("W".into(), move_to_string(mv))),
                _ => None,
            }
        })
        .collect())
}

pub fn write_as_sgf(stones: &Vec<(String, String)>, path: &str) -> anyhow::Result<()> {
    let game_tree = stones.iter().rev().fold(None, |acc, (player, mv)| {
        let mv_parsed = string_to_move(mv);
        let sgf_move = match mv_parsed {
            sgf_parse::go::Move::Pass => "".into(),
            sgf_parse::go::Move::Move(point) => ((point.x + ('a' as u8)) as char).to_string() + ((point.y + ('a' as u8)) as char).to_string().as_str(),
        };
        let properties = vec![
            sgf_parse::go::Prop::new(player.clone(), vec![sgf_move]),
        ];

        let Some(child) = acc else {
            return Some(sgf_parse::SgfNode::new(properties, vec![], false));
        };
        Some(sgf_parse::SgfNode::new(properties, vec![child], false))
    });

    let node = game_tree.ok_or_else(|| anyhow::Error::msg("bad board"))?;

    Ok(fs::write(path, node.serialize())?)
}

#[cfg(test)]
mod test {
    use crate::sgf::{move_to_string, string_to_move};


    #[test]
    fn test_move_string_conversions() {
        for x in 0..19u8 {
            for y in 0..19u8 {
                let mv = sgf_parse::go::Move::Move(sgf_parse::go::Point{x, y});
                assert_eq!(mv, string_to_move(&move_to_string(&mv)));
            }
        }
    }
}