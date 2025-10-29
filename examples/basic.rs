use wv::*;

fn main() {
    let mut wv = Webview::create_no_win(false);
    wv.set_title("Webview Window").unwrap();
    wv.set_size(800, 300, SizeHint::Fixed).unwrap();
    wv.navigate("https://www.wikipedia.com").unwrap();
    wv.run().unwrap();
}
