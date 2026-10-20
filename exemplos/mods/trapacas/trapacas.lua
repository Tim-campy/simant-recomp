-- Cheat menu to test the game (mod API 0.17; 0.26 for castes, scents, ants in the nests and the save;
-- 0.43 for the texts in data/idiomas; docs/api-mods.md; README.md next to this file).
-- F12 opens the menu. Each item calls an API action, which uses the original game's routine for it
-- (the start of the rain, _XferPatch, _YellowDeath, the Mystery...). Every cheat used shows up in the
-- message strip ("Cheat: ...") and is counted in the panel; since API 0.26 the count goes to the save's
-- side file.
--
-- Texts: every text this mod shows comes from data/idiomas/<language>.json inside the mod (keys
-- "mod.trapacas.*"), read with traduzir(); English is the fallback and pt-BR is a second example.

local TECLA = "F12"

-- The mod's text `chave`, in the game's language, with the {name} parameters of `params`.
local function t(chave, params)
  return traduzir("mod.trapacas." .. chave, params)
end

-- Menu items: text and function. "-" is a separator.
local itens = {}
local acoes = {}

local function item(chave, f)
  itens[#itens + 1] = t("item." .. chave)
  acoes[#itens] = f
end

local function separador()
  itens[#itens + 1] = "-"
end

-- The mod's own state (the count and the panel go to the save's side file: ao_salvar, API 0.26).
local painel_ligado = false
local procurando_evento = false
local usadas = 0
local proxima_casa = 0

local function avisar(texto)
  mostrar_mensagem(t("aviso", { texto = texto }))
end

-- Free cells around the yellow ant (on the surface or in the nest she is in).
local function em_volta(estado, f)
  local a = estado:amarela()
  if not a.plano then
    return 0
  end
  local feitas = 0
  for dx = -1, 1 do
    for dy = -1, 1 do
      if (dx ~= 0 or dy ~= 0) and f(a.plano, a.x + dx, a.y + dy) then
        feitas = feitas + 1
      end
    end
  end
  return feitas
end

local function atualizar_painel(estado)
  if not painel_ligado then
    definir_painel(nil)
    return
  end
  local s, r = estado:sementes()
  local chuva = estado:chuva()
  local m = estado:menino()
  local px, py = estado:patch()
  local p, v = estado:colonia("preta"), estado:colonia("vermelha")
  local licao, dada = estado:licao()
  local a = estado:amarela()
  local linhas = {
    string.format("GameTime %d  Cycle %d  %s", estado:tempo(), estado:ciclo(), estado:tipo_jogo()),
    string.format("S %04X  R %08X", s, r),
    t("painel.patch", { x = px, y = py, plano = a.plano or "-", ax = a.x, ay = a.y, saude = a.saude }),
    t("painel.colonias", { pretas = p.adultos, saude_p = p.saude, vermelhas = v.adultos, saude_v = v.saude }),
    t("painel.rainhas", { pretas = p.rainhas_guardadas, vermelhas = v.rainhas_guardadas }),
    t("painel.chuva", { chove = t(chuva.chovendo and "sim" or "nao"), contagem = chuva.contagem, lencol = chuva.lencol }),
    string.format("_FlyAway %d / %d", estado:revoada("preta"), estado:revoada("vermelha")),
    t("painel.menino", {
      no = m.no,
      onde = t(m.dentro_de_casa and "painel.em_casa" or "painel.la_fora"),
      cortando = m.cortando_grama and t("painel.cortando") or "",
    }),
    t(dada and "painel.licao_dada" or "painel.licao", { licao = licao }),
    t("painel.usadas", { n = usadas }),
  }
  definir_painel(table.concat(linhas, "\n"))
end

-- Rain ------------------------------------------------------------------------------------
item("comecar_chuva", function(e) e:comecar_chuva() end)
item("parar_chuva", function(e) e:parar_chuva() end)
separador()
-- The boy ---------------------------------------------------------------------------------
item("menino_grama", function(e) e:menino_cortar_grama() end)
item("menino_inseticida", function(e) e:menino_inseticida() end)
item("patch_cozinha", function(e)
  e:povoar_patch("preta", 0, 12, 3)
  e:trocar_patch(0, 12)
end)
item("patch_casa", function(e)
  -- House patches: index x*16+y from 0 to 37 (quintal.md, _MakeMap).
  proxima_casa = (proxima_casa + 1) % 38
  local x, y = proxima_casa // 16, proxima_casa % 16
  e:povoar_patch("preta", x, y, 3)
  e:trocar_patch(x, y)
  return t("detalhe.patch", { x = x, y = y })
end)
item("patch_ao_lado", function(e)
  local px, py = e:patch()
  local x = px < 11 and px + 1 or px - 1
  e:povoar_patch("preta", x, py, 3)
  e:trocar_patch(x, py)
  return t("detalhe.patch", { x = x, y = py })
end)
separador()
-- Colonies and mating flight --------------------------------------------------------------
item("guardar_rainhas", function(e) e:guardar_rainhas("preta", 3) end)
item("revoada_vermelha", function(e)
  e:criar_reprodutores("vermelha", 25)
  e:armar_revoada("vermelha")
end)
item("revoada_preta", function(e)
  e:criar_reprodutores("preta", 25)
  e:armar_revoada("preta")
end)
separador()
-- Creatures -------------------------------------------------------------------------------
item("gato", function(e) e:trazer_gato() end)
item("cachorro", function(e) e:latir() end)
item("aranha", function(e) e:trazer_aranha() end)
item("formiga_leao", function(e)
  local a = e:amarela()
  if a.plano ~= "superficie" then
    return t("detalhe.fora_da_superficie")
  end
  e:por_formiga_leao(math.min(math.max(a.x + 4, 1), 126), math.min(math.max(a.y, 1), 62))
end)
separador()
-- The yellow ant --------------------------------------------------------------------------
item("luta", function(e)
  -- API 0.26: por_formiga works on the surface and in the nests.
  local n = em_volta(e, function(plano, x, y) return e:por_formiga(plano, "vermelha", x, y, "soldado") end)
  return n > 0 and t("detalhe.soldados", { n = n }) or t("detalhe.sem_lugar")
end)
item("comida", function(e)
  local n = em_volta(e, function(plano, x, y) return e:colocar_comida(plano, x, y) end)
  return t("detalhe.pedacos", { n = n })
end)
item("curar", function(e) e:saude_amarela(100) end)
item("matar", function(e) e:matar_amarela() end)
separador()
-- Colony (API 0.26: direct writes, like the Caste/Behavior windows and the tools) ----------
item("castas_soldados", function(e) e:definir_castas(0, 0, 1) end)
item("comportamento_forragear", function(e) e:definir_comportamento(1, 0, 0) end)
item("alarme", function(e)
  local a = e:amarela()
  if a.plano ~= "superficie" then
    return t("detalhe.cheiro_so_na_superficie")
  end
  local n = em_volta(e, function(_, x, y) return e:depositar_cheiro("alarme", x, y) end)
  return t("detalhe.celulas", { n = n })
end)
item("soldados_no_ninho", function(e)
  local postos = 0
  for x = 1, 62 do
    for y = 1, 62 do
      if postos < 5 and e:celula("ninho_preto", x, y) < 8 and e:por_formiga("ninho_preto", "preta", x, y, "soldado") then
        postos = postos + 1
      end
    end
  end
  return t("detalhe.postos", { n = postos })
end)
separador()
-- Time ------------------------------------------------------------------------------------
item("avancar_100", function() avancar_ticks(100) end)
item("avancar_500", function() avancar_ticks(500) end)
item("avancar_1000", function() avancar_ticks(1000) end)
item("proximo_evento", function()
  procurando_evento = true
  avancar_ticks(5000)
end)
separador()
-- Game ------------------------------------------------------------------------------------
item("licao", function(e) e:avancar_licao() end)
item("vitoria", function(e) e:terminar_jogo("vitoria") end)
item("derrota", function(e) e:terminar_jogo("derrota") end)
item("misterio", function(e) e:misterio() end)
item("painel", function(e)
  painel_ligado = not painel_ligado
  atualizar_painel(e)
  return t(painel_ligado and "detalhe.ligado" or "detalhe.desligado")
end)

-- Hooks -----------------------------------------------------------------------------------

function ao_iniciar(estado)
  definir_menu({ titulo = t("titulo"), tecla = TECLA, itens = itens })
  registrar(t("registro.menu", { tecla = TECLA, api = API_VERSAO, idioma = idioma() }))
end

function ao_menu(indice, texto, estado)
  local f = acoes[indice]
  if not f then
    return
  end
  local detalhe = f(estado)
  usadas = usadas + 1
  avisar(detalhe and t("com_detalhe", { texto = texto, detalhe = detalhe }) or texto)
  registrar(t("registro.usada", { texto = texto, detalhe = detalhe or "" }))
  atualizar_painel(estado)
end

function ao_evento(ev, estado)
  if not procurando_evento then
    return
  end
  if ev.tipo == "mensagem" or ev.tipo == "musica" or ev.tipo == "fim_de_jogo" or ev.tipo == "mensagem_faixa" then
    procurando_evento = false
    parar_avanco()
    mostrar_mensagem(t("parou", { tick = estado:tempo(), tipo = ev.tipo, id = tostring(ev.id or ev.indice or "") }))
  end
end

-- API 0.26: the cheat count and the panel go to NAME.ANT.mods.json, next to the save
-- (the .ANT stays the original's).
function ao_salvar(estado)
  return { usadas = usadas, painel = painel_ligado }
end

function ao_carregar(dados, estado)
  usadas = dados.usadas or 0
  painel_ligado = dados.painel or false
  atualizar_painel(estado)
  if usadas > 0 then
    avisar(t("ja_teve", { n = usadas }))
  end
end

function depois_do_tick(estado)
  if painel_ligado then
    atualizar_painel(estado)
  end
end
