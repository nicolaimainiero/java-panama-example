# Project Panama: Eine verbesserte Schnittstelle für Native Code

In manchen Situationen kann es sein, dass aus einer Java Applikation heraus ein bestehendes Programm, z.B. ein hardwarenaher Treiber aufgerufen werden muss. Oder dass aus Performance Gründen auf nativen Code ausgewichen werden muss. Bisher musste dabei auf JNI zurückgegriffen werden. Mit dem Project Panama bündelt das OpenJDK verschiedene JEPs, um eine einheitliche und leicht verwendbare API dafür zu bieten.
## Rust/Java Interoperabilität
Neben den vielen anderen JEPs, wie zum Beispiel der Vector API (JEP-338) um in Java plattformunabhängig Code zu schreiben der auf Vektorhardware wie SEE, AVX oder ARM Neon ausgeführt wird, soll hier die Interoperabilität mit Rust demonstriert werden. 
Rust ist eine Multiparadigmen-Systemprogrammiersprache, die mit dem Ziel entwickelt wurde, um sicher, nebenläufig und praxisnah zu sein. Sicher bezieht sich hier besonders auf die Vermeidung von Programmierfehlern die zu Speicherzugriffsfehlern oder Pufferüberläufen führen. In diesem Beispiel soll ein minimales Rust Programm, das zwei Ganzzahlen addiert aus einem Java Programm heraus aufgerufen werden.

## Project Panama Early-Access Builds
Um mit der neuen API und den Tools arbeiten zu können wird ein JDK 19 mit den entsprechenden Funktionen benötigt. Dieser werden vorkompiliert von Oracle unter [^1] zur Verfügung gestellt. Mit Hilfe von sdkman [^2] kann einfach das entpackte plattformspezifische Paket aktiviert werden.
```sh
$ sdk install java 19-panama <pfad zum extrahierten Archiv>/zulu-17.jdk/Contents/Home
```
Danach kann für die aktuelle Shell einfach auf den early-access build gewechselt werden, indem man es wie mit sdkman gewohnt aktiviert.
```sh
$ sdk use java 19-panama
```
Nachdem nun eine Umgebung mit dem early-access build zur Verfügung steht, wird noch die Rust Toolchain benötigt, um ein Rustprogramm zu kompilieren.

## Rust Setup und Anwendung
Für Rust gibt es mit ``rustup`` ein Kommandozeilen Tool [^3], das sowohl den Compiler installiert als auch als Versionsmanager fungiert. In jedem unix-artigen Betriebssystem kann über 
```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
`rustup` installiert werden. Nach der Installation kann ein neues Rust Projekt angelegt werden, indem ein neues Verzeichnis `rust-adder` erzeugt wird und in diesem `cargo init --lib` aufgerufen wird. Es wird unter anderem eine `src/lib.rs` angelegt deren Inhalt mit folgendem ersetzt werden muss.
```rust
#[no_mangle]
pub extern "C" fn rust_add_numbers(a: i32, b: i32) -> i32 {
    return a + b;
}
```
Die Funktion `rust_add_numbers` addiert einfach die beiden übergebenen Zahlen. Rust kennt im Gegensatz zu C oder C++ keine Headerdateien. Für die Interaktion mit Java werden jedoch Headerdateien benötigt. Um diese zu erzeugen, muss die `Cargo.toml` angepasst und ein Generator geschrieben werden.
```toml
[build-dependencies]
cbindgen = "0.24.3"

[lib]
crate_type = ["cdylib"]

[package]
name = "rust-adder"
version = "0.1.0"
edition = "2021"

# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

[dependencies]
```
Das Tool `cbindgen`` wird dann von dem folgenden Programm verwendet, um eine Headerdatei zu erzeugen.

```rust
extern crate cbindgen;

use std::env;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_language(cbindgen::Language::C)
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file("lib.h");
}
```
Dieses Programm erzeugt eine `lib.h`` Datei, nachdem es mit ``cargo build`` ausgeführt worden ist. Diese enthält die Signatur der exportierten Funktion und alle notwendigen Abhängigkeiten.
```c
#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

int32_t rust_add_numbers(int32_t a, int32_t b);
```

## jextract und die Java Integration
Mit dem Projekt Panama kommt ein neues Kommandozeilenwerkzeug `jextract`, dass den notwendigen Code zur Integration in Java erzeugen kann. Mit dem Aufruf
```sh
jextract -d classes -t de.sidion.rust -l rust-adder -- lib.h
```
werden unter `classes/de/sidion/rust` die notwendigen .class-Dateien abgelegt. Falls unter macOS folgende Fehlermeldung angezeigt wird
`lib.h:4:10: fatal error: 'stdlib.h' file not found` muss beim Aufruf der Pfad zum macOS SDK mit angegeben werden. Der Aufruf lautet dann wie folgt
```sh
jextract -I /Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/include  -d classes -t de.sidion.rust -l rust-adder -- lib.h
```
Jetzt kann die in Rust implementierte Funktion aus Java heraus aufgerufen werden. Dazu wird eine einfach Main-Klasse erzeugt, die die Funktion aufruft und das Ergebnis ausgibt.

```java
import static de.sidion.rust.lib_h.*;

public class Main {
    public static void main(String[] args){
        System.out.println("23 + 42 = " + rust_add_numbers(23,42));
    }
}
```

Seit JDK 11 kann diese Klasse mit folgendem Befehl nun direkt ausgeführt werden, ohne vorher kompiliert zu werden. 
```sh
java --add-modules jdk.incubator.foreign \                             
   --enable-native-access=ALL-UNNAMED \
   -Djava.library.path=./target/debug \
   -cp classes \
   Main.java
```
Wenn alles funktioniert hat erscheint folgende Ausgabe in der Shell
```sh
WARNING: Using incubator modules: jdk.incubator.foreign
warning: using incubating module(s): jdk.incubator.foreign
1 warning
23 + 42 = 65
```

## Stolperstellen
Wer mit dem aktuellen early-access build auf einem Mac mit ARM CPU experimentiert wird beim Ausführen der Main-Klasse einen Fehler erhalten, der sich über die fehlende Native Bibliothek beschwert.
```sh
WARNING: Using incubator modules: jdk.incubator.foreign
warning: using incubating module(s): jdk.incubator.foreign
1 warning
Exception in thread "main" java.lang.UnsatisfiedLinkError: no rust_adder in java.library.path: ./target/debug
	at java.base/java.lang.ClassLoader.loadLibrary(ClassLoader.java:2434)
	at java.base/java.lang.Runtime.loadLibrary0(Runtime.java:848)
	at java.base/java.lang.System.loadLibrary(System.java:2015)
	at de.sidion.rust.RuntimeHelper.<clinit>(RuntimeHelper.java:41)
	at de.sidion.rust.constants$27.<clinit>(constants$27.java:14)
	at de.sidion.rust.lib_h.rust_add_numbers(lib_h.java:2844)
	at Main.main(Main.java:5)
```
Das liegt daran, dass der early-access build im Moment nur für die x64 Architektur zur Verfügung gestellt wird und sich daher die JVM und die native Bibliothek in ihren Ziel-Architekturen unterscheiden. Dies stellt aber kein Problem dar, da Rust die Möglichkeit der Cross-Kompoilation unterstützt. Indem also die passende Ziel-Architektur beim Kompilieren der nativen Bibliothek angegeben wird, funktioniert auch der Aufruf aus Java heraus. Dazu muss das entsprechende Ziel mit `rustup target add x86_64-apple-darwin` hinzugefügt und mit `cargo build --target x86_64-apple-darwin` gebaut werden. Der Aufruf der Main-Klasse ändert sich dann wie folgt

```sh
java --add-modules jdk.incubator.foreign \          
   --enable-native-access=ALL-UNNAMED \
   -Djava.library.path=./target/x86_64-apple-darwin/debug \
   -cp classes \
   Main.java
```

## Fazit
Die verbesserte Integration führt zu einer sehr vereinfachten Interaktion zwischen Java und nativen Code. Das Kommandozeilenwerkzeug jextract übernimmt die aufwendige Konfiguration, wie das Laden der Bibliothek und das Definieren von Methoden-Stubs. Das ermöglicht die schnelle Integration von bestehenden Bibliotheken, wie z.B. Treiber, um auf deren Funktionalität zurückzugreifen oder die Implementierung besonders performancekritischer Teile in einer hardwarenahen Sprache.

## Referenzen

[^1]: 	[Project Panama Early-Access Builds](https://jdk.java.net/panama/)
[^2]: 	[The Software Development Kit Manager](https://sdkman.io/)
[^3]: 	[Rust - Getting started](https://www.rust-lang.org/learn/get-started)
