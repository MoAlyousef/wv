use wv::*;

fn main() {
    let mut wv = Webview::create_no_win(false);
    wv.set_size(800, 600, SizeHint::Min).unwrap();
    wv.set_title("Webview Window").unwrap();
    wv.navigate("https://www.wikipedia.com").unwrap();
    wv.run().unwrap();
}
