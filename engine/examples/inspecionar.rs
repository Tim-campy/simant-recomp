//! Ferramenta de inspeção: lista os objetos de uma janela (tipo 0 + nomes do tipo 1) ou as strings
//! de uma lista tipo 4, lidas dos bancos do jogo (com os mods aplicados).
//! Uso: cargo run -p engine --example inspecionar -- <pasta do jogo> janela <id> | strings <id> | bitmap <id>
use engine::core::telas::carregar_janela;
use engine::formats::banco::Chave;
use engine::formats::textos::ListaStrings;
use engine::recursos::Recursos;
use std::path::Path;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let bancos = ["SHARED", "WINGANT", "SOUND"].map(String::from);
    let r = Recursos::abrir(Path::new(&a[1]), &bancos, Path::new("data"), None).unwrap();
    let id: u16 = a[3].parse().unwrap();
    match a[2].as_str() {
        "janela" => {
            let j = carregar_janela(&r, id).unwrap();
            for (i, o) in j.objetos.iter().enumerate() {
                let rt = o.retangulo;
                println!(
                    "{i:3} {:24} tipo {:2} ({:4},{:4})-({:4},{:4}) flags {:04x} cor {:3}/{:3} {:?}",
                    o.nome, o.tipo, rt.esq, rt.topo, rt.dir, rt.base, o.flags, o.cor, o.cor_selecionado, o.conteudo
                );
            }
        }
        "strings" => {
            let d = r.objeto(Chave { tipo: 4, id }).unwrap();
            for (i, s) in ListaStrings::decodificar(&d).unwrap().strings.iter().enumerate() {
                println!("{i:3} {:?}", String::from_utf8_lossy(s));
            }
        }
        "posicoes" => {
            let d = r.objeto(Chave { tipo: 9, id }).unwrap();
            for (i, p) in engine::formats::janela::decodificar_posicoes(&d).unwrap().iter().enumerate() {
                println!("{i:3} {p:?}");
            }
        }
        "bitmap" => {
            let d = r.objeto(Chave { tipo: 2, id }).unwrap();
            let b = engine::formats::bitmap::Bitmap::decodificar(&d).unwrap();
            println!("{} × {}, cabeçalho {:02x?}, {} bytes", b.largura, b.altura, &d[..12], d.len());
            for y in 0..b.altura {
                let l: String = (0..b.largura).map(|x| format!("{:x}", b.pixels[y * b.largura + x])).collect();
                println!("{l}");
            }
        }
        "layout" => {
            // Campos de layout de cada objeto (docs/spec/formatos/janela.md, "Cálculo do layout"):
            // deslocamentos (0x08), referências (0x10) e modos (0x18) das quatro coordenadas.
            let d = r.objeto(Chave { tipo: 0, id }).unwrap();
            let u = |o: usize| u16::from_le_bytes([d[o], d[o + 1]]);
            let n = u(0x0c) as usize;
            println!("flags da janela (0x1c, 0x1d): {:02x} {:02x}", d[0x1c], d[0x1d]);
            let mut o = 0x2c + 4 * n;
            for i in 0..n {
                let desl: Vec<i16> = (0..4).map(|k| u(o + 8 + 2 * k) as i16).collect();
                let refs: Vec<String> = (0..4).map(|k| format!("{:04x}", u(o + 0x10 + 2 * k))).collect();
                let modos: Vec<u16> = (0..4).map(|k| u(o + 0x18 + 2 * k)).collect();
                println!("{i:3} tipo {:2} byte28 {:3} desl {desl:?} ref {refs:?} modos {modos:?}", d[o + 0x21], d[o + 0x28]);
                o += u(o + 0x22) as usize;
            }
        }
        _ => eprintln!("use janela <id>, layout <id>, strings <id> ou bitmap <id>"),
    }
}
