use wv::*;

fn main() {
    let mut wv = Webview::create_no_win(false);
    wv.set_title("Webview Window");
    wv.set_size(400, 300, SizeHint::Fixed);
    wv.navigate("https://www.wikipedia.com");
    wv.run();
}
