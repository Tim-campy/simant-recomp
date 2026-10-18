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
    // exemplos/mods tem também o mod de trapaças (só lê o estado nos ganchos de tick).
    assert_eq!(s.situacao(), vec![("exemplo-lua".to_string(), true), ("trapacas".to_string(), true)]);
    let mut e = estado_quick();
    e.ambiente.proxima_comida = -1; // sem a alimentação periódica do jogo, para isolar o script
    let mut regras = RegrasOriginais::default();
    s.ao_iniciar(&mut e);
    assert!(s.retirar_registro()[0].contains("chuva de comida ligada; API 0.20, jogo rapido"));
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

// ---------------------------------------------------------------------------------------------
// API 0.17: menu, painel, ações e avanço de ticks (pela Partida, que executa os pedidos)
// ---------------------------------------------------------------------------------------------

mod api017 {
    use super::*;
    use engine::core::avisos::ConfigAvisos;
    use engine::core::navegacao::{Evento as EventoTecla, Navegacao};
    use engine::core::partida::{ConfigPartida, Partida};
    use engine::core::tela_jogo::PosicaoVista;
    use engine::formats::save::{JogoSalvo, LayoutSave};
    use engine::rules::amarela::TabelasAmarela;
    use engine::rules::estado::{ConfigSimulacao, Evento};
    use engine::rules::quintal::{self, Relogios, TabelasQuintal};
    use engine::scripts::TeclaMod;

    fn ler(nome: &str) -> String {
        fs::read_to_string(raiz().join("data").join(nome)).unwrap()
    }

    /// Partida do QUICK2.ANT (Jogo Rápido), sem scripts.
    pub(super) fn partida() -> Partida {
        let layout = LayoutSave::carregar(&raiz().join("data/save_layout.json")).unwrap();
        let jogo = JogoSalvo::abrir(&layout, &raiz().join("tests/referencias/saves/QUICK2.ANT")).unwrap();
        let estado = Estado::de_save(&jogo).unwrap();
        let nav = Navegacao::do_save(&jogo, PosicaoVista::AoCarregar, 25, 19).unwrap();
        Partida::nova(
            estado,
            nav,
            RegrasOriginais::default(),
            TabelasAmarela::padrao().clone(),
            ConfigSimulacao::de_json(&ler("simulacao.json")).unwrap(),
            ConfigPartida::de_json(&ler("partida.json")).unwrap(),
            ConfigAvisos::de_json(&ler("mensagens.json")).unwrap(),
        )
    }

    /// Partida com um script cujo `ao_menu(i)` roda `acoes[i]`.
    fn partida_com(acoes: &[&str]) -> Partida {
        let corpo: String = acoes.iter().enumerate().map(|(i, a)| format!("[{}] = function(e) {a} end,\n", i + 1)).collect();
        let codigo = format!(
            "local acoes = {{\n{corpo}}}\n\
             function ao_iniciar(e) definir_menu({{titulo = 'T', tecla = 'F12', itens = {{'a'}}}}) end\n\
             function ao_menu(i, texto, e) acoes[i](e) end"
        );
        let mut p = partida();
        p.ligar_scripts(scripts(&codigo));
        p
    }

    fn escolher(p: &mut Partida, i: usize) {
        p.escolher_no_menu_de_mod("teste", i);
    }

    fn rodar(p: &mut Partida, n: u32) {
        for _ in 0..n {
            p.rodar_tick();
        }
    }

    #[test]
    fn tecla_do_menu_exige_funcao_ou_ctrl_alt() {
        assert!(TeclaMod::de_texto("F12").is_ok());
        let t = TeclaMod::de_texto("Ctrl+Shift+T").unwrap();
        assert!(t.casa(&EventoTecla::Tecla { nome: "T".into(), ctrl: true, shift: true, alt: false }));
        assert!(!t.casa(&EventoTecla::Tecla { nome: "T".into(), ctrl: true, shift: false, alt: false }));
        assert_eq!(t.texto(), "Ctrl+Shift+T");
        assert!(TeclaMod::de_texto("T").is_err(), "letra sozinha é do jogo (amarela, trapaças)");
        assert!(TeclaMod::de_texto("Shift+1").is_err());
        assert!(TeclaMod::de_texto("Hyper+F1").is_err());
    }

    #[test]
    fn menu_e_painel_definidos_pelo_script() {
        let mut s = scripts(
            "function ao_iniciar(e) definir_menu({titulo = 'Teste', tecla = 'F11', itens = {'um', '-', 'dois'}}) \
             definir_painel('linha 1\\nlinha 2') end \
             function ao_menu(i, texto, e) registrar(i, texto, e:tempo()) definir_painel(nil) end",
        );
        let mut e = estado_quick();
        s.ao_iniciar(&mut e);
        let menus = s.menus();
        assert_eq!(menus.len(), 1);
        assert_eq!(menus[0].1.itens, vec!["um", "-", "dois"]);
        assert_eq!(s.paineis(), vec![("teste".to_string(), vec!["linha 1".to_string(), "linha 2".to_string()])]);
        s.retirar_registro();
        s.ao_menu("teste", 3, &mut e);
        assert_eq!(s.retirar_registro(), vec!["[teste] 3\tdois\t0".to_string()]);
        assert!(s.paineis().is_empty());
        // Tecla inválida: erro desliga o mod.
        let mut s = scripts("function ao_iniciar(e) definir_menu({titulo = 'x', tecla = 'Q', itens = {'a'}}) end");
        s.ao_iniciar(&mut e);
        assert_eq!(s.ativo("teste"), Some(false));
    }

    #[test]
    fn sem_usar_o_menu_o_jogo_fica_igual() {
        let mut com = partida_com(&["e:comecar_chuva()"]);
        let mut sem = partida();
        rodar(&mut com, 200);
        rodar(&mut sem, 200);
        assert!(com.estado == sem.estado, "definir um menu não muda o tick");
    }

    #[test]
    fn chuva_comeca_com_os_sorteios_do_original_e_a_agua_sobe() {
        let mut p = partida_com(&["e:comecar_chuva()", "e:parar_chuva()"]);
        let mut esperado = p.estado.clone();
        let rel = Relogios::do_port(&esperado, TabelasQuintal::padrao());
        quintal::comecar_chuva(&mut esperado, TabelasQuintal::padrao(), rel);
        escolher(&mut p, 1);
        assert_eq!(p.estado.ambiente.chovendo, 1);
        assert!((150..300).contains(&p.estado.ambiente.chuva_contagem));
        assert_eq!((p.estado.s, p.estado.r), (esperado.s, esperado.r), "S e R como o começo da chuva de _DoSimYard");
        let lencol = p.estado.ambiente.lencol;
        rodar(&mut p, 120);
        assert!(p.estado.ambiente.lencol < lencol, "_WaterTable sobe nos ninhos ({lencol} → {})", p.estado.ambiente.lencol);
        // Chover de novo é recusado; parar desliga no tick seguinte.
        escolher(&mut p, 1);
        assert!(p.retirar_registro_scripts().iter().any(|l| l.contains("já está chovendo")));
        escolher(&mut p, 2);
        rodar(&mut p, 1);
        assert_eq!(p.estado.ambiente.chovendo, 0);
    }

    #[test]
    fn avancar_ticks_roda_sem_o_relogio_e_parar_interrompe() {
        let mut p = partida_com(&["avancar_ticks(150)", "parar_avanco()"]);
        let t0 = p.estado.partida.tempo;
        escolher(&mut p, 1);
        assert_eq!(p.ticks_de_mods_pendentes(), 150);
        while p.ticks_de_mods_pendentes() > 0 {
            p.avancar(0);
        }
        assert_eq!(p.estado.partida.tempo, t0 + 150);
        escolher(&mut p, 1);
        escolher(&mut p, 2);
        assert_eq!(p.ticks_de_mods_pendentes(), 0);
    }

    #[test]
    fn bichos_amarela_e_colonias() {
        let mut p = partida_com(&[
            "e:trazer_gato()",
            "e:latir()",
            "e:trazer_aranha()",
            "e:saude_amarela(7)",
            "e:saude_colonia('vermelha', 3)",
            "e:guardar_rainhas('preta', 2)",
            "e:povoar_patch('vermelha', 3, 9, 20)",
            "e:matar_amarela()",
            "e:menino_inseticida()",
            "e:menino_cortar_grama()",
        ]);
        p.estado.aranha.ligada = 0;
        p.estado.bichos.gato_ligado = 0;
        for i in 1..=7 {
            escolher(&mut p, i);
        }
        let e = &p.estado;
        assert_eq!(e.bichos.gato_ligado, 1);
        assert_eq!(e.aranha.ligada, 1);
        assert_eq!(e.amarela.saude, 7);
        assert_eq!(e.vermelha.saude, 3);
        assert_eq!(e.preta.rainhas_guardadas, 2);
        assert_eq!(e.vermelha.populacao_quintal.0[3 * 16 + 9], 20);
        p.retirar_eventos();
        escolher(&mut p, 8);
        assert!(p.retirar_eventos().contains(&Evento::Mensagem(10040)), "_YellowDeath(8): morreu de fome");
        escolher(&mut p, 9);
        assert_eq!(p.estado.menino.no_numero, 32);
        escolher(&mut p, 10);
        let m = &p.estado.menino;
        assert_eq!((m.no_numero, m.presente, m.cortando_grama), (0x26, 3, 1));
        assert_eq!((m.px, m.py), p.estado.ambiente.patch);
    }

    #[test]
    fn latido_pede_o_som_do_cachorro() {
        let mut p = partida_com(&["e:latir()"]);
        let s0 = p.estado.s;
        escolher(&mut p, 1);
        assert_ne!(p.estado.s, s0, "SRand4 e o atraso do latido");
        let latidos = &TabelasQuintal::padrao().sons.latidos;
        assert!(p.retirar_eventos().iter().any(|ev| matches!(ev, Evento::Som(id) if latidos.contains(id))));
    }

    #[test]
    fn reprodutores_e_revoada_armada() {
        let mut p = partida_com(&["e:criar_reprodutores('vermelha', 25) e:armar_revoada('vermelha')", "e:armar_revoada('preta')"]);
        escolher(&mut p, 1);
        assert_eq!(p.estado.vermelha.revoada, 200);
        let machos = p.estado.superficie.lista.ativas().filter(|(_, f)| f.tipo & 0xF8 == 0xA0 && f.modo == 14).count();
        assert!(machos > 0, "machos alados em pré-voo");
        // O jogo faz o resto: pré-voo → decolagem (15) com _FlyAway > 100, e saem voando (16).
        let mut decolaram = false;
        for _ in 0..200 {
            p.rodar_tick();
            decolaram |= p.estado.superficie.lista.ativas().any(|(_, f)| f.tipo & 0x80 != 0 && f.modo >= 15 && f.tipo & 0x78 == 0x20);
        }
        assert!(decolaram, "alados decolam");
        escolher(&mut p, 2);
        assert!(p.retirar_registro_scripts().iter().any(|l| l.contains("sem machos")));
    }

    #[test]
    fn fim_de_jogo_licao_e_misterio() {
        let mut p = partida_com(&["e:terminar_jogo('vitoria')", "e:avancar_licao()", "e:misterio()"]);
        escolher(&mut p, 2);
        assert!(p.retirar_registro_scripts().iter().any(|l| l.contains("só no tutorial")));
        let r0 = p.estado.r;
        escolher(&mut p, 3);
        assert!(p.ultimo_misterio.is_some());
        assert_ne!(p.estado.r, r0, "um RRand(15)");
        escolher(&mut p, 1);
        assert_eq!((p.estado.partida.fim_de_jogo, p.estado.partida.pretas_venceram), (1, 1));
        p.rodar_tick();
        assert!(p.retirar_eventos().contains(&Evento::DialogoFimDeJogo));
    }

    #[test]
    fn argumentos_invalidos_sao_erro() {
        for codigo in ["e:trocar_patch(12, 0)", "e:saude_amarela(101)", "e:povoar_patch('azul', 1, 1, 1)", "avancar_ticks(0)", "e:terminar_jogo('empate')"] {
            let mut p = partida_com(&[codigo]);
            escolher(&mut p, 1);
            assert_eq!(p.scripts.as_ref().unwrap().ativo("teste"), Some(false), "{codigo}");
        }
    }
}
