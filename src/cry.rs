#[cfg(feature = "cries")]
use anyhow::{Context, Result};
#[cfg(feature = "cries")]
use directories::ProjectDirs;
#[cfg(feature = "cries")]
use reqwest::blocking::Client;
#[cfg(feature = "cries")]
use std::cell::RefCell;
#[cfg(feature = "cries")]
use std::collections::HashMap;
#[cfg(feature = "cries")]
use std::path::{Path, PathBuf};
#[cfg(feature = "cries")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "cries")]
use std::thread::JoinHandle;

/// 鳴き声の取得に許す時間。鳴き声1件は約7KBなので、これを超えるなら
/// 回線かGitHub側の問題であって、待っても鳴る見込みは薄い
#[cfg(feature = "cries")]
const CRY_FETCH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);

/// 鳴き声の取得・再生を管理するサービス
#[cfg(feature = "cries")]
pub struct CryService {
    cache_dir: PathBuf,
    client: Client,
    base_url: String,
    id_map: HashMap<String, u32>,
    /// 音声デバイスは開くのに約88msかかる（実測）。起動時に裏で開き始め、
    /// 実際に鳴らすスレッドが受け取る。デバイスが無い環境では None になり、
    /// 再生は黙ってスキップされる。
    sink: Arc<Mutex<SinkSlot>>,
    /// 取得〜再生を行っているスレッド。プロセス終了前に wait() で待つ
    playing: RefCell<Option<JoinHandle<()>>>,
    /// 再生中のプレイヤー。選び直したときに前の鳴き声を止めるために共有する
    player: Arc<Mutex<Option<rodio::Player>>>,
}

/// 起動時に投げたデバイスオープンの、開いている最中／開き終わった状態
#[cfg(feature = "cries")]
enum SinkSlot {
    Opening(JoinHandle<Option<rodio::MixerDeviceSink>>),
    Ready(Option<rodio::MixerDeviceSink>),
}

#[cfg(feature = "cries")]
impl SinkSlot {
    /// デバイスが開き終わるのを待って参照を渡す。待つのは最初の1回だけ
    fn with<R>(slot: &Mutex<Self>, f: impl FnOnce(&rodio::MixerDeviceSink) -> R) -> Option<R> {
        let mut slot = slot.lock().ok()?;
        if matches!(*slot, SinkSlot::Opening(_)) {
            let SinkSlot::Opening(handle) = std::mem::replace(&mut *slot, SinkSlot::Ready(None))
            else {
                unreachable!()
            };
            *slot = SinkSlot::Ready(handle.join().unwrap_or(None));
        }
        match &*slot {
            SinkSlot::Ready(sink) => sink.as_ref().map(f),
            SinkSlot::Opening(_) => unreachable!(),
        }
    }
}

#[cfg(feature = "cries")]
impl CryService {
    /// 新しいCryServiceインスタンスを作成
    pub fn new() -> Result<Self> {
        use crate::data::DataLoader;

        let project_dirs = ProjectDirs::from("", "", "poke-lookup")
            .or_else(|| ProjectDirs::from("dev", "poke-lookup", "poke-lookup"))
            .context("Failed to determine project directories")?;

        let cache_dir = project_dirs.data_dir().join("cries");

        if !cache_dir.exists() {
            std::fs::create_dir_all(&cache_dir).with_context(|| {
                format!(
                    "Failed to create cry cache directory: {}",
                    cache_dir.display()
                )
            })?;
        }

        // 取得スレッドは終了前に join されるので、応答が返らないと CLI 自体が
        // 止まる。鳴き声は付加機能なので、待たせるくらいなら諦める
        let client = Client::builder()
            .user_agent(concat!("poke-lookup/", env!("CARGO_PKG_VERSION")))
            .timeout(CRY_FETCH_TIMEOUT)
            .build()
            .context("Failed to create HTTP client")?;

        // Load Pokemon ID mapping
        let loader = DataLoader::new()?;
        let dictionary = loader.load_dictionary()?;
        let id_map = dictionary
            .entries
            .iter()
            .filter_map(|entry| entry.id.map(|id| (entry.en.clone(), id)))
            .collect();

        Ok(Self {
            cache_dir,
            client,
            base_url: "https://raw.githubusercontent.com/PokeAPI/cries/main".to_string(),
            id_map,
            sink: Arc::new(Mutex::new(SinkSlot::Opening(std::thread::spawn(
                Self::open_sink,
            )))),
            playing: RefCell::new(None),
            player: Arc::new(Mutex::new(None)),
        })
    }

    /// 音声デバイスを開く。失敗しても鳴らないだけなので None を返す
    fn open_sink() -> Option<rodio::MixerDeviceSink> {
        let mut sink = rodio::DeviceSinkBuilder::open_default_sink().ok()?;
        // drop 時の警告ログが CLI 出力に混ざるのを防ぐ
        sink.log_on_drop(false);
        Some(sink)
    }

    /// 英名からポケモンIDを取得
    pub fn get_pokemon_id(&self, english_name: &str) -> Option<u32> {
        self.id_map.get(english_name).copied()
    }

    /// ポケモンIDからローカルキャッシュの鳴き声パスを取得
    pub fn get_cry_path(&self, pokemon_id: u32) -> PathBuf {
        self.cache_dir.join(format!("{}.ogg", pokemon_id))
    }

    /// ポケモンの鳴き声の取得と再生をバックグラウンドで開始する
    ///
    /// ダウンロードもデバイスの準備待ちもスレッド側でやるので、呼び出し側は
    /// すぐ戻って英名の出力やスプライト描画に進める。
    /// 鳴り終わりはプロセス終了前に [`Self::wait`] で待つ。
    ///
    /// 取得も再生も失敗しうるが、鳴らないだけで英名を出すという本来の目的には
    /// 影響しないため、エラーは返さず握り潰す。
    pub fn play_cry_for_pokemon(&self, english_name: &str) {
        let Some(pokemon_id) = self.get_pokemon_id(english_name) else {
            return;
        };

        // 前の鳴き声は止めてから次に移る。鳴り終わるまで待つと、ESC で選び直した
        // 直後に「捨てたはずのポケモンの声を最後まで聞かされる」ことになる
        self.stop();

        let cry_path = self.get_cry_path(pokemon_id);
        let url = self.cry_url(pokemon_id);
        let client = self.client.clone();
        let sink = Arc::clone(&self.sink);
        let player_slot = Arc::clone(&self.player);

        *self.playing.borrow_mut() = Some(std::thread::spawn(move || {
            // 取得もデバイス待ちも、失敗したら黙って諦める。鳴らないだけで
            // 英名を出すという本来の目的には影響しない
            if download_if_missing(&client, &url, &cry_path).is_err() {
                return;
            }
            SinkSlot::with(&sink, |sink| {
                play_and_wait(sink, &cry_path, &player_slot);
            });
        }));
    }

    /// 再生中の鳴き声を止めてスレッドの後始末をする
    fn stop(&self) {
        if let Some(player) = self.player.lock().ok().and_then(|mut p| p.take()) {
            player.stop();
        }
        self.wait();
    }

    /// 再生中の鳴き声が鳴り終わるまで待つ
    ///
    /// 待たずにプロセスが終わると音が途中で切れる。再生していなければ即座に返る。
    pub fn wait(&self) {
        if let Some(handle) = self.playing.borrow_mut().take() {
            let _ = handle.join();
        }
    }

    /// 鳴き声の配信URL
    fn cry_url(&self, pokemon_id: u32) -> String {
        format!("{}/cries/pokemon/latest/{}.ogg", self.base_url, pokemon_id)
    }

    // テストでは音声デバイスを開かない（CIランナーには存在しないため）
    #[cfg(test)]
    pub fn for_test(cache_dir: PathBuf, id_map: HashMap<String, u32>) -> Self {
        Self {
            cache_dir,
            client: Client::new(),
            base_url: "test://mock".to_string(),
            id_map,
            sink: Arc::new(Mutex::new(SinkSlot::Ready(None))),
            playing: RefCell::new(None),
            player: Arc::new(Mutex::new(None)),
        }
    }
}

/// 未取得なら鳴き声をダウンロードして保存する
#[cfg(feature = "cries")]
fn download_if_missing(client: &Client, url: &str, cry_path: &Path) -> Result<()> {
    if cry_path.exists() {
        return Ok(());
    }

    let response = client
        .get(url)
        .send()
        .with_context(|| format!("Failed to fetch cry: {}", url))?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Failed to download cry from {}: HTTP {}",
            url,
            response.status()
        ));
    }

    let content = response.bytes().context("Failed to read cry data")?;

    // 一時ファイルに書いてから rename する。ダウンロードはバックグラウンドスレッドで
    // 走っており、Ctrl-C や process::exit でいつでも巻き添えで死にうる。直接書くと
    // 途中まで書けたファイルが残り、以後 exists() が true になって永久に無音になる
    let tmp_path = cry_path.with_extension(format!("ogg.{}.part", std::process::id()));
    std::fs::write(&tmp_path, content)
        .with_context(|| format!("Failed to save cry to {}", tmp_path.display()))?;

    std::fs::rename(&tmp_path, cry_path).with_context(|| {
        format!(
            "Failed to move {} into place at {}",
            tmp_path.display(),
            cry_path.display()
        )
    })?;

    Ok(())
}

/// 鳴き声を再生して鳴り終わるまで待つ
///
/// 再生中のプレイヤーを `player_slot` に置くことで、呼び出し側から stop() できる。
#[cfg(feature = "cries")]
fn play_and_wait(
    sink: &rodio::MixerDeviceSink,
    cry_path: &Path,
    player_slot: &Mutex<Option<rodio::Player>>,
) {
    use std::fs::File;
    use std::io::BufReader;

    let Ok(file) = File::open(cry_path) else {
        return;
    };

    // 拡張子は .ogg だが実体は MP3。Decoder は中身で判定するのでそのまま渡せる
    let Ok(player) = rodio::play(sink.mixer(), BufReader::new(file)) else {
        return;
    };

    // 呼び出し側が stop() できるよう、プレイヤーを共有スロットに預ける
    if let Ok(mut slot) = player_slot.lock() {
        *slot = Some(player);
    } else {
        return;
    }

    // player.sleep_until_end() ではなくポーリングで待つ。プレイヤーはスロットに
    // 預けてしまっており、ロックを保持したまま待つと stop() 側が固まるため
    loop {
        let done = match player_slot.lock() {
            Ok(slot) => match slot.as_ref() {
                Some(player) => player.empty(),
                // stop() で取り上げられた＝中断された
                None => true,
            },
            Err(_) => true,
        };
        if done {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

// 再生そのものは音声デバイスを必要とするため CI では検証できない。
// ここではダウンロード・キャッシュ・ID解決のみをテストする。
#[cfg(test)]
#[cfg(feature = "cries")]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_cry_path() {
        let temp_dir = tempdir().unwrap();
        let service = CryService::for_test(temp_dir.path().to_path_buf(), HashMap::new());

        assert_eq!(service.get_cry_path(25), temp_dir.path().join("25.ogg"));
        assert_eq!(service.get_cry_path(1), temp_dir.path().join("1.ogg"));
    }

    #[test]
    fn test_cry_url_uses_latest() {
        let temp_dir = tempdir().unwrap();
        let service = CryService::for_test(temp_dir.path().to_path_buf(), HashMap::new());

        assert_eq!(
            service.cry_url(25),
            "test://mock/cries/pokemon/latest/25.ogg"
        );
    }

    #[test]
    fn test_get_pokemon_id() {
        let temp_dir = tempdir().unwrap();
        let mut id_map = HashMap::new();
        id_map.insert("Pikachu".to_string(), 25);
        id_map.insert("Bulbasaur".to_string(), 1);

        let service = CryService::for_test(temp_dir.path().to_path_buf(), id_map);

        assert_eq!(service.get_pokemon_id("Pikachu"), Some(25));
        assert_eq!(service.get_pokemon_id("Bulbasaur"), Some(1));
        assert_eq!(service.get_pokemon_id("Unknown"), None);
    }

    #[test]
    fn test_play_cry_for_unknown_pokemon_is_noop() {
        let temp_dir = tempdir().unwrap();
        let service = CryService::for_test(temp_dir.path().to_path_buf(), HashMap::new());

        // 辞書に無ければスレッドも立てずに戻る
        service.play_cry_for_pokemon("Unknown");
        service.wait();
    }

    #[test]
    fn test_download_skipped_when_cached() {
        let temp_dir = tempdir().unwrap();
        let cry_path = temp_dir.path().join("25.ogg");
        fs::write(&cry_path, b"cached_audio").unwrap();

        // キャッシュがあれば到達不能なURLでも成功し、中身も上書きされない
        let result = download_if_missing(
            &Client::new(),
            "http://127.0.0.1:1/cries/pokemon/latest/25.ogg",
            &cry_path,
        );

        assert!(result.is_ok());
        assert_eq!(fs::read(&cry_path).unwrap(), b"cached_audio");
    }

    #[test]
    fn test_download_success() {
        use httpmock::prelude::*;

        let server = MockServer::start();
        let mock_audio = b"ID3\x04\x00\x00\x00\x00\x00\x00dummy audio data".to_vec();

        let mock = server.mock(|when, then| {
            when.method(GET).path("/cries/pokemon/latest/25.ogg");
            then.status(200)
                .header("content-type", "audio/ogg")
                .body(&mock_audio);
        });

        let temp_dir = tempdir().unwrap();
        let cry_path = temp_dir.path().join("25.ogg");

        let result = download_if_missing(
            &Client::new(),
            &server.url("/cries/pokemon/latest/25.ogg"),
            &cry_path,
        );

        assert!(result.is_ok());
        assert_eq!(fs::read(&cry_path).unwrap(), mock_audio);
        mock.assert();
    }

    #[test]
    fn test_download_leaves_no_partial_file_behind() {
        use httpmock::prelude::*;

        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/cries/pokemon/latest/25.ogg");
            then.status(200).body(b"audio");
        });

        let temp_dir = tempdir().unwrap();
        let cry_path = temp_dir.path().join("25.ogg");

        download_if_missing(
            &Client::new(),
            &server.url("/cries/pokemon/latest/25.ogg"),
            &cry_path,
        )
        .unwrap();

        // rename 後に .part が残っていないこと
        let leftovers: Vec<_> = fs::read_dir(temp_dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains(".part"))
            .collect();
        assert!(leftovers.is_empty(), "partial files left: {:?}", leftovers);
        assert_eq!(fs::read(&cry_path).unwrap(), b"audio");
    }

    #[test]
    fn test_download_failure_leaves_no_file() {
        use httpmock::prelude::*;

        let server = MockServer::start();
        let _mock = server.mock(|when, then| {
            when.method(GET).path("/cries/pokemon/latest/9999.ogg");
            then.status(404).body("Not Found");
        });

        let temp_dir = tempdir().unwrap();
        let cry_path = temp_dir.path().join("9999.ogg");

        let result = download_if_missing(
            &Client::new(),
            &server.url("/cries/pokemon/latest/9999.ogg"),
            &cry_path,
        );

        assert!(result.is_err());
        // 失敗時に壊れたファイルを残さない
        assert!(!cry_path.exists());
    }
}
