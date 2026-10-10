//! Installed cinematic definitions from MENU.INI [page3].
use std::collections::BTreeMap;

/// A timed subtitle. Frames are one-based, start-inclusive and stop-exclusive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovieSubtitle {
    pub always: bool,
    pub color: [u8; 3],
    pub start: u32,
    pub stop: u32,
    pub text: String,
}

/// Filename, forward skip points and optional subtitles for one cinematic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovieDefinition {
    pub name: String,
    pub keys: Vec<u32>,
    pub subtitles: Vec<MovieSubtitle>,
}

/// Parse sequential cinematic definitions. First case-insensitive fields win.
/// Names are single filenames, never paths. Commas inside subtitle text survive.
pub fn parse_menu_movies(content: &str) -> Result<Vec<MovieDefinition>, String> {
    let mut fields = BTreeMap::new();
    let mut active = false;
    for line in content.lines() {
        let line = line.split(';').next().unwrap_or_default().trim();
        if line.starts_with('[') {
            active = line.eq_ignore_ascii_case("[page3]");
        } else if active && let Some((key, value)) = line.split_once('=') {
            fields
                .entry(key.trim().to_ascii_lowercase())
                .or_insert(value.trim());
        }
    }
    let error = |key: &str| format!("MENU.INI [page3] {key}: invalid cinematic definition");
    let mut movies = Vec::new();
    while let Some(name) = fields.get(&format!("movie_name[{}]", movies.len())) {
        let index = movies.len();
        if name.is_empty()
            || !name.is_ascii()
            || name.contains(['/', '\\', ':'])
            || *name == "."
            || *name == ".."
        {
            return Err(error(&format!("movie_name[{index}]")));
        }
        let key = format!("movie_keys[{index}]");
        let keys: Vec<u32> = fields
            .get(&key)
            .filter(|v| !v.is_empty())
            .map(|v| {
                v.split(',')
                    .map(|n| n.trim().parse().map_err(|_| error(&key)))
                    .collect()
            })
            .transpose()?
            .unwrap_or_default();
        if keys.contains(&0) || keys.windows(2).any(|v| v[0] >= v[1]) {
            return Err(error(&key));
        }
        let mut subtitles = Vec::new();
        while let Some(value) = fields.get(&format!("movie{index}_text[{}]", subtitles.len())) {
            let key = format!("movie{index}_text[{}]", subtitles.len());
            let parts: Vec<_> = value.splitn(7, ',').collect();
            if parts.len() != 7 {
                return Err(error(&key));
            }
            let number = |i: usize| parts[i].trim().parse::<u32>().map_err(|_| error(&key));
            let always = number(0)?;
            let color = [number(1)?, number(2)?, number(3)?];
            let start = number(4)?;
            let stop = number(5)?;
            if always > 1
                || color.iter().any(|v| *v > 255)
                || start == 0
                || start >= stop
                || !parts[6].is_ascii()
            {
                return Err(error(&key));
            }
            subtitles.push(MovieSubtitle {
                always: always != 0,
                color: color.map(|v| v as u8),
                start,
                stop,
                text: parts[6].trim().into(),
            });
        }
        movies.push(MovieDefinition {
            name: name.to_ascii_uppercase().chars().take(15).collect(),
            keys,
            subtitles,
        });
    }
    for key in fields.keys() {
        if let Some(index) = key
            .strip_prefix("movie_name[")
            .and_then(|v| v.strip_suffix(']'))
            && index.parse::<usize>().is_ok_and(|i| i >= movies.len())
        {
            return Err(error(key));
        }
    }
    Ok(movies)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn subtitles_preserve_commas_and_first_fields_win() {
        let movies = parse_menu_movies("[PAGE3]\nmovie_name[0]=INTRO.SMK\nmovie_name[0]=OTHER.SMK\nmovie_keys[0]=2,10\nmovie0_text[0]=1,255,0,9,2,10,Hello, Cyrus.").unwrap();
        assert_eq!(movies[0].name, "INTRO.SMK");
        assert_eq!(movies[0].keys, [2, 10]);
        assert_eq!(
            movies[0].subtitles[0],
            MovieSubtitle {
                always: true,
                color: [255, 0, 9],
                start: 2,
                stop: 10,
                text: "Hello, Cyrus.".into()
            }
        );
        for text in [
            "movie_name[0]=../bad",
            "movie_name[1]=gap",
            "movie_name[0]=x\nmovie_keys[0]=5,2",
            "movie_name[0]=x\nmovie0_text[0]=0,999,0,0,1,2,x",
        ] {
            assert!(parse_menu_movies(&format!("[page3]\n{text}")).is_err());
        }
    }
}
