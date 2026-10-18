//! Scripts Lua de mods (docs/api-mods.md, "Scripts Lua"): sandbox, limites, erro que desliga só o
//! mod, o mod de exemplo (`exemplos/mods/exemplo-lua`) e a garantia de que, sem scripts, o tick não muda.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use engine::formats::save::{Formiga, Lista};
use engine::recursos::Recursos;
use engine::rules::estado::Estado;
use engine::rules::experimental::TabelasExperimental;
use engine::rules::tick::{tick, RegrasOriginais};
use engine::scripts::{Gancho, Limites, Scripts};

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Um mod por item: (nome, código de um único arquivo).
fn scripts_com(mods: &[(&str, &str)], limites: Limites) -> Scripts {
    let fontes = mods.iter().map(|(n, c)| (n.to_string(), Ok(vec![("main.lua".to_string(), c.to_string())]))).collect();
    Scripts::de_fontes(fontes, TabelasExperimental::padrao().clone(), limites)
}

fn scripts(codigo: &str) -> Scripts {
    scripts_com(&[("teste", codigo)], Limites::default())
}

fn estado_quick() -> Estado {
    let mut e = Estado::vazio();
    e.partida.tipo_jogo = 1;
    e
}

#[test]
fn sandbox_nao_tem_io_os_nem_carga_de_codigo() {
    let mut s = scripts(
        r#"
        local proibidos = {}
        for _, n in ipairs({"io", "os", "package", "require", "debug", "load", "loadfile", "dofile", "collectgarbage"}) do
          if _G[n] ~= nil then proibidos[#proibidos + 1] = n end
        end
        if string.dump ~= nil then proibidos[#proibidos + 1] = "string.dump" end
        registrar("proibidos:" .. table.concat(proibidos, ","))
        function ao_iniciar(estado) os.execute("echo oi") end
        "#,
    );
    assert_eq!(s.retirar_registro(), vec!["[teste] proibidos:".to_string()]);
    let mut e = estado_quick();
    s.ao_iniciar(&mut e);
    assert_eq!(s.ativo("teste"), Some(false), "os.execute não existe: erro desliga o script");
    let reg = s.retirar_registro().join("\n");
    assert!(reg.contains("script desligado em ao_iniciar"), "{reg}");
    assert_eq!(s.retirar_mensagens().len(), 1, "aviso na faixa");
}

#[test]
fn bytecode_e_recusado() {
    // Um chunk binário (começa com ESC) não carrega: só texto.
    let mut s = scripts("\x1bLua");
    assert_eq!(s.ativo("teste"), None, "o mod não chega a ser carregado");
    assert!(s.retirar_registro()[0].contains("script desligado"));
}

#[test]
fn limite_de_instrucoes_interrompe_laco_infinito() {
    let limites = Limites { instrucoes: 100_000, tempo: Duration::from_secs(10), ..Limites::default() };
    let mut s = scripts_com(
        &[
            ("laco", "function depois_do_tick(e) while true do end end"),
            // pcall não pode engolir o erro de limite.
            ("teimoso", "function depois_do_tick(e) while true do pcall(function() while true do end end) end end"),
        ],
        limites,
    );
    let mut e = estado_quick();
    s.depois_do_tick(&mut e);
    assert_eq!(s.ativo("laco"), Some(false));
    assert_eq!(s.ativo("teimoso"), Some(false));
    assert!(s.retirar_registro().iter().all(|l| l.contains("limite excedido")));
    assert!(!s.tem_gancho(Gancho::DepoisDoTick));
}

#[test]
fn limite_de_memoria_desliga_o_script() {
    let limites = Limites { memoria: 2 * 1024 * 1024, ..Limites::default() };
    let mut s = scripts_com(&[("guloso", "function antes_do_tick(e) local t = {} for i = 1, 1e9 do t[i] = i end end")], limites);
    let mut e = estado_quick();
    s.antes_do_tick(&mut e);
    assert_eq!(s.ativo("guloso"), Some(false));
}

#[test]
fn erro_desliga_so_o_script_que_falhou() {
    let mut s = scripts_com(
        &[
            ("bom", "n = 0 function depois_do_tick(e) n = n + 1 registrar(n) end"),
            ("ruim", "function depois_do_tick(e) error('falhei') end"),
            ("argumento", "function depois_do_tick(e) e:celula('lua', 0, 0) end"),
        ],
        Limites::default(),
    );
    let mut e = estado_quick();
    for _ in 0..3 {
        s.depois_do_tick(&mut e);
    }
    assert_eq!(s.ativo("bom"), Some(true));
    assert_eq!(s.ativo("ruim"), Some(false));
    assert_eq!(s.ativo("argumento"), Some(false), "plano desconhecido é erro");
    let reg = s.retirar_registro();
    assert_eq!(reg.iter().filter(|l| l.starts_with("[bom]")).count(), 3);
    assert!(reg.iter().any(|l| l.starts_with("[ruim] script desligado") && l.contains("falhei")));
    assert_eq!(s.retirar_mensagens().len(), 2);
}

#[test]
fn leitura_do_estado() {
    let mut e = estado_quick();
    e.partida.tempo = 1234;
    e.preta.castas = [1, 2, 3, 4, 5, 1];
    e.preta.populacao = 15;
    e.superficie.mapa.definir(5, 6, 0x48);
    e.adicionar_formiga(Lista::A, Formiga { x: 7, y: 8, modo: 2, tipo: 0x90 | 3, s: 0 });
    e.amarela.plano = 2;
    e.amarela.x = 10;
    let mut s = scripts(
        r#"
        function ao_iniciar(e)
          local c = e:colonia("preta")
          local a = e:amarela()
          local f = e:formigas("superficie")
          local l, h = e:dimensoes("ninho_preto")
          registrar(e:tempo(), e:tipo_jogo(), c.adultos, c.soldados, c.rainha, e:celula("superficie", 5, 6),
            tostring(e:celula("superficie", 500, 0)), a.plano, a.x, #f, f[1].cor, f[1].grupo, f[1].direcao,
            e:ocupante("superficie", 7, 8), l, h)
        end
        "#,
    );
    s.ao_iniciar(&mut e);
    assert_eq!(
        s.retirar_registro(),
        vec!["[teste] 1234\trapido\t15\t3\t1\t72\tnil\tninho_preto\t10\t1\tvermelha\t2\t3\t147\t64\t64".to_string()]
    );
}

#[test]
fn escrita_validada_e_sem_mexer_nos_geradores() {
    let mut e = estado_quick();
    e.s.definir_semente(0x4321);
    e.superficie.mapa.definir(2, 2, 0x60); // obstáculo: nem comida nem pedra
    let (s0, r0) = (e.s, e.r);
    let mut s = scripts(
        r#"
        function ao_iniciar(e)
          registrar(e:colocar_comida("superficie", 1, 1), e:colocar_comida("superficie", 2, 2),
            e:colocar_comida("superficie", -1, 0), e:colocar_pedra(3, 3), e:colocar_pedra(3, 3),
            e:colocar_formiga("preta", 4, 4), e:colocar_formiga("vermelha", 4, 4),
            e:colocar_comida("ninho_preto", 10, 10), aleatorio(1, 6), math.random(10))
          e:definir_velocidade(3)
        end
        "#,
    );
    s.ao_iniciar(&mut e);
    let linha = s.retirar_registro().remove(0);
    assert!(linha.starts_with("[teste] true\tfalse\tfalse\ttrue\tfalse\ttrue\tfalse\ttrue\t"), "{linha}");
    assert_eq!(e.superficie.mapa.em(1, 1), Some(0x48));
    assert_eq!(e.ambiente.comida_superficie, 1);
    assert_eq!(e.preta.comida, 1);
    assert_eq!(e.superficie.vida.em(4, 4), Some(16));
    assert_eq!(e.superficie.lista.indice, 1);
    assert_eq!(e.partida.velocidade, 3);
    assert_eq!((e.s, e.r), (s0, r0), "o script não consome S nem R");
    assert_eq!(s.ativo("teste"), Some(true));
}

#[test]
fn objeto_estado_so_vale_durante_a_chamada() {
    let mut s = scripts("function ao_iniciar(e) guardado = e end function depois_do_tick(e) guardado:tempo() end");
    let mut e = estado_quick();
    s.ao_iniciar(&mut e);
    assert_eq!(s.ativo("teste"), Some(true));
    s.depois_do_tick(&mut e);
    assert_eq!(s.ativo("teste"), Some(false), "usar o estado de uma chamada anterior é erro");
}

#[test]
fn aleatorio_do_script_e_deterministico_por_mod() {
    let sorteios = || {
        let mut s = scripts("local t = {} for i = 1, 5 do t[i] = aleatorio(1, 1000) end registrar(table.concat(t, ','))");
        s.retirar_registro()
    };
    assert_eq!(sorteios(), sorteios());
}

/// Recursos com só o `data/` do port e o diretório de mods dado (sem os bancos do jogo).
fn recursos_com_mods(mods: &std::path::Path) -> Recursos {
    Recursos::abrir(&raiz(), &[], &raiz().join("data"), Some(mods)).unwrap()
}

#[test]
fn mod_de_exemplo_chove_comida_a_cada_500_ticks() {
    let r = recursos_com_mods(&raiz().join("exemplos/mods"));
    let mut s = Scripts::carregar(&r, Limites::default()).unwrap().expect("exemplos/mods/exemplo-lua tem scripts");
    assert_eq!(s.situacao(), vec![("exemplo-lua".to_string(), true)]);
    let mut e = estado_quick();
    e.ambiente.proxima_comida = -1; // sem a alimentação periódica do jogo, para isolar o script
    let mut regras = RegrasOriginais::default();
    s.ao_iniciar(&mut e);
    assert!(s.retirar_registro()[0].contains("chuva de comida ligada; API 0.12, jogo rapido"));
    let mut sem_script = e.clone();
    for _ in 0..499 {
        s.antes_do_tick(&mut e);
        tick(&mut e, &mut regras);
        s.depois_do_tick(&mut e);
        tick(&mut sem_script, &mut RegrasOriginais::default());
    }
    assert_eq!(e, sem_script, "antes do tick 500 o exemplo não muda nada");
    assert!(s.retirar_mensagens().is_empty());
    tick(&mut e, &mut regras);
    s.depois_do_tick(&mut e);
    let comida = e.ambiente.comida_superficie;
    assert!(comida > 0 && comida <= 30, "comida = {comida}");
    // Chão vira 0x48 (um pedaço); cair de novo na mesma célula soma uma unidade (0x49, 0x4A).
    let pedacos: i32 = e.superficie.mapa.dados.iter().filter(|&&v| (0x48..=0x4A).contains(&v)).map(|&v| i32::from(v) - 0x47).sum();
    assert_eq!(pedacos, i32::from(comida), "cada pedaço contado em _FoodA está no mapa");
    let msgs = s.retirar_mensagens();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].texto, format!("Choveu comida! ({comida} pedaços)"));
}

#[test]
fn sem_scripts_nada_e_carregado_e_o_tick_fica_igual() {
    let vazio = std::env::temp_dir().join(format!("simant_mods_lua_vazio_{}", std::process::id()));
    let _ = fs::remove_dir_all(&vazio);
    fs::create_dir_all(vazio.join("so-dados")).unwrap();
    fs::write(vazio.join("so-dados/mod.json"), r#"{"nome":"so-dados","versao":"1.0"}"#).unwrap();
    let r = recursos_com_mods(&vazio);
    assert!(Scripts::carregar(&r, Limites::default()).unwrap().is_none(), "mod sem \"scripts\" não cria máquina Lua");

    // Um mod com script que só lê não muda o tick.
    let mut s = scripts("function antes_do_tick(e) local _ = e:colonia('preta') end function depois_do_tick(e) local _ = e:formigas('superficie') end");
    let mut a = estado_quick();
    a.s.definir_semente(77);
    let mut b = a.clone();
    for _ in 0..200 {
        s.antes_do_tick(&mut a);
        tick(&mut a, &mut RegrasOriginais::default());
        s.depois_do_tick(&mut a);
        tick(&mut b, &mut RegrasOriginais::default());
    }
    assert_eq!(a, b);
    let _ = fs::remove_dir_all(&vazio);
}

#[test]
fn caminho_de_script_invalido_desliga_o_mod() {
    let dir = std::env::temp_dir().join(format!("simant_mods_lua_caminho_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("fujao")).unwrap();
    fs::write(dir.join("fujao/mod.json"), r#"{"nome":"fujao","versao":"1.0","scripts":["../fora.lua"]}"#).unwrap();
    let r = recursos_com_mods(&dir);
    let mut s = Scripts::carregar(&r, Limites::default()).unwrap().unwrap();
    assert!(s.situacao().is_empty());
    assert!(s.retirar_registro()[0].contains("caminho de script inválido"));
    let _ = fs::remove_dir_all(&dir);
}
