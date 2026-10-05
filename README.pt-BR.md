# Revenant

[English](README.md) | **Português (Brasil)**

Revenant é um projeto autoral de jogo e preservação de software. Combina um
cliente em Godot, um servidor em Rust e persistência em PostgreSQL. O servidor
valida as ações e as recompensas; o histórico de eventos permite reconstruir
sessões e verificar a compatibilidade com versões antigas do próprio jogo.

O escopo **M0–M38 está concluído**, com a versão **0.2.0** mantida. O código atual
inclui uma experiência solo local com campanha, exploração e desafios.

## O que está incluído

- Campanha de seis capítulos, com registros opcionais e diferentes epílogos.
- Cinco armas, dez módulos de equipamento e três comissões com recompensas únicas.
- Oito contratos de desafio, modificadores, seis maestrias e três títulos que não aumentam o poder do personagem.
- Interface em inglês e português brasileiro, controles remapeáveis e suporte a controle.
- Menus em 100%, 125% ou 150%, alto contraste, legendas de sons e opções para reduzir movimento e flashes.
- Histórico persistente, reconstrução de sessões e compatibilidade preservada com o cliente V1.

A branch `main` contém o código até M38. Os downloads da seção **Releases**
correspondem às versões publicadas anteriormente; o envio deste código não criou
um novo instalador. O pacote privado de M38 para Windows e Linux, seus backups
e suas credenciais ficam fora do Git. Os detalhes técnicos estão no
[registro de conclusão de M33–M38](docs/roadmap-m33-m38-experience-expansion.md)
e no [guia do pacote local de M38](docs/operations/m38-local-archive.md), em inglês.

## Requisitos

Para executar a partir do código:

- Linux com Bash, ou WSL2 para executar os comandos de terminal no Windows.
- Git, `curl` e OpenSSL.
- Docker com Docker Compose.
- Godot **4.7.1** para abrir o cliente do jogo.

O Compose compila o servidor e o Inspector em contêineres. Para desenvolver ou
executar as verificações fora deles, instale também Rust **1.97.1** com `rustfmt`
e `clippy`, Node.js **20** com npm, Python **3.10 ou superior** e GNU Make.

## Instalação e execução local

### 1. Obter o código

Execute no terminal Linux ou WSL2:

```bash
git clone https://github.com/iangama/revenant.git
cd revenant
```

Se já tiver o repositório, entre na pasta existente.

### 2. Preparar as credenciais

Na primeira instalação, gere os arquivos locais usados pelo banco e pelo servidor:

```bash
scripts/m30-generate-secrets.sh current
```

O script cria os arquivos fora do repositório, com acesso restrito ao seu usuário.
Mantenha-os no sistema de arquivos Linux, inclusive no WSL2. Nas próximas
inicializações, reutilize esses arquivos: o script recusa sobrescrever uma
geração existente. Não é necessário gerar novas senhas toda vez que for jogar.

### 3. Iniciar os serviços

Em cada nova sessão de terminal, informe a pasta das credenciais e inicie o Compose:

```bash
export REVENANT_SECRETS_DIR="${XDG_STATE_HOME:-$HOME/.local/state}/revenant/m30-secrets/current"
docker compose -f infra/docker-compose.yml up --build -d
docker compose -f infra/docker-compose.yml ps
```

A primeira inicialização compila o servidor e o Inspector. Aguarde PostgreSQL,
gateway e Inspector aparecerem como saudáveis (`healthy`) antes de conferir a resposta:

```bash
curl http://127.0.0.1:8080/health
```

O servidor deve responder com o status `ok`. O Inspector fica em
[http://127.0.0.1:4173](http://127.0.0.1:4173) e permite consultar sessões e
eventos sem alterá-los. Os serviços usam endereços locais da sua máquina.

Se precisar definir um caminho personalizado, copie `.env.example` para `.env`,
configure `REVENANT_SECRETS_DIR` com um caminho absoluto no Linux e use
`docker compose --env-file .env -f infra/docker-compose.yml up --build -d`.
O arquivo `.env` não deve entrar no Git.

### 4. Abrir o jogo

Abra `client/game/project.godot` no Godot 4.7.1 e pressione **F5**. Na tela de
entrada, informe um identificador local e conecte ao host `127.0.0.1`, porta
**7000**. Use o mesmo identificador para acessar seu progresso salvo.

Nas configurações, selecione **Português (Brasil)**. Também é possível ajustar
o volume, o modo de exibição, a escala da interface, as orientações na tela,
o contraste, as legendas de sons e a redução de movimento e flashes.

Com Rust instalado, `cargo run -p revenant-bot` executa um cliente opcional de
terminal para verificar a conexão.

### 5. Encerrar e voltar depois

Para parar os serviços e manter os dados salvos, execute no terminal configurado:

```bash
docker compose -f infra/docker-compose.yml stop
```

Para voltar, repita a etapa de inicialização dos serviços e abra o projeto no
Godot. Reutilize as credenciais existentes.

## Como jogar

### Modos de jogo

Na tela de entrada, escolha uma opção em **Modo de jogo**:

- **Campanha — continuar / retomar:** inicia ou retoma a história de seis capítulos a partir dos pontos de progresso salvos. A primeira conclusão de cada capítulo concede um fragmento e 100 XP.
- **Treino:** permite revisitar capítulos já concluídos, sem conceder itens ou XP.
- **Quadro de desafios:** carrega seus registros; depois, escolha um contrato em Modo de jogo para iniciar a tentativa. Os desafios não concedem itens ou XP, e os títulos de maestria não aumentam o poder de combate.
- **Operação avulsa:** abre a missão do relé, com exploração e encontros opcionais.

Na operação avulsa, derrote o drone inicial e avance até `x=6` para abrir o
núcleo e enfrentar o Warden. Antes de seguir ao núcleo, é possível explorar
rotas, registros e encontros opcionais. A campanha orienta o avanço pelos seus
próprios objetivos; os registros opcionais não são obrigatórios para concluí-la.

### Controles padrão

| Ação | Teclado e mouse |
| --- | --- |
| Mover | **WASD**, setas ou botões direcionais na tela |
| Atacar | **Espaço**, clique no inimigo ativo ou botão de ataque na tela |
| Alternar alvo | **V** |
| Selecionar Pulse Rifle, Arc Sidearm ou Coil Lance | **1**, **2** ou **3** |
| Percorrer as cinco armas | **Q / F** ou botões de arma na tela |
| Abrir a oficina de módulos | **M** |
| Escolher uma rota disponível | **R** |
| Ler um registro próximo | **E** |
| Abrir o arquivo de descobertas | **J** |
| Consultar orientações | **H** |
| Focar a barra de ações | **T** |
| Abrir configurações | **P**, ou **Escape** durante a sessão |
| Navegar pelos menus | **Tab**, setas e **Enter**; **Escape** fecha o menu aberto |

No controle, o analógico esquerdo move o personagem; os botões de ombro
percorrem as armas. O direcional navega pelos menus, o botão inferior confirma
e o botão da direita volta ou fecha. Ataques por teclado ou controle usam o
inimigo ativo, sem exigir mira com o cursor.

A aba de controles permite remapear teclas e botões. Ao atribuir uma tecla
ocupada, as duas atribuições são trocadas. Os botões da interface mostram os
atalhos configurados. As pistas sonoras têm correspondentes visuais ou textuais.

### Equipamento e progresso

A oficina mostra as cinco armas, os efeitos dos módulos e permite salvar até
três módulos para a próxima entrada. Rail Driver e Scatter Caster fazem parte
do arsenal, além das três armas com atalhos numéricos. O servidor valida a posse
do equipamento, as alterações e as recompensas.

Os registros de campanha, desafios e maestrias são persistidos no banco.
O inventário e a experiência exibidos pelo cliente refletem o estado confirmado
pelo servidor. Consulte os objetivos de cada modo para distinguir recompensas
de primeira conclusão, treino e metas de desafio.

## Estrutura do projeto

| Pasta | Conteúdo |
| --- | --- |
| `client/game` | Cliente Godot, interface, apresentação e controles |
| `runtime` | Servidor Rust, combate, campanha, desafios, persistência e replay |
| `web/control-panel` | Inspector de sessões e eventos |
| `infra` | Contêineres e configuração da infraestrutura local |
| `scripts` | Atividades Lua e ferramentas de operação |
| `tools` | Clientes de terminal e ferramentas de desenvolvimento |
| `tests` | Verificações de funcionamento e integridade |
| `archive/clients/v1` | Cliente V1 preservado para verificar compatibilidade |
| `docs` | Arquitetura, protocolos e registros das etapas concluídas |

## Desenvolvimento e documentação técnica

As mudanças devem seguir as [regras de validação do projeto](AGENTS.md).
Para documentação, basta revisar o texto, os links e o diff. Use verificações
direcionadas para alterações pequenas. A suíte completa continua disponível:

```bash
make check
```

Ela verifica formatação, lint, testes e compilação do Rust, o Inspector,
o fluxo do Godot, persistência, replay e compatibilidade V1. Antes de executá-la,
configure o banco de testes e as ferramentas locais conforme a
[documentação técnica em inglês](README.md#validate). As ferramentas de banco
exigem `DATABASE_URL` ou `DATABASE_URL_FILE`; o Compose normal mantém o
PostgreSQL sem porta publicada no host. Não aponte testes de banco para dados
de jogo que deseja preservar.

Os documentos técnicos e históricos permanecem em inglês:

- [Arquitetura dos protocolos e compatibilidade](docs/protocol/README.md).
- [Backup e restauração do PostgreSQL](docs/operations/postgresql-backup.md).
- [Ferramentas e scripts](scripts/README.md).
- [Inspector](docs/inspector/README.md).
- [Escopo e conclusão de M33–M38](docs/roadmap-m33-m38-experience-expansion.md).
- [Pacote privado de M38 para Windows e Linux](docs/operations/m38-local-archive.md).

Nomes de arquivos, comandos, variáveis de ambiente e identificadores de código
permanecem em inglês. Este README reúne as orientações essenciais em português.
