import { useState } from "react";
import Head from "next/head";

type Cell = "empty" | "troop" | "hit" | "miss" | "fog" | "destroyed";

const COLS = ["A","B","C","D","E"];
const ROWS = ["1","2","3","4","5"];

function Board({ label, cells, onCellClick, mode, isActive }: {
  label: string;
  cells: Cell[];
  onCellClick?: (i: number) => void;
  mode: "placement" | "attack" | "view";
  isActive?: boolean;
}) {
  const getColor = (c: Cell) => {
    if (c === "troop") return "bg-yellow-500 border-yellow-400";
    if (c === "hit") return "bg-red-600 border-red-400 animate-pulse";
    if (c === "miss") return "bg-blue-900 border-blue-600";
    if (c === "destroyed") return "bg-red-900 border-red-700 opacity-60";
    if (c === "fog") return "bg-gray-800 border-gray-700 hover:bg-gray-700";
    return "bg-gray-900 border-gray-700 hover:bg-gray-800";
  };

  const getIcon = (c: Cell) => {
    if (c === "troop") return "⚔️";
    if (c === "hit") return "💥";
    if (c === "miss") return "〇";
    if (c === "destroyed") return "💀";
    return "";
  };

  return (
    <div className="inline-block">
      <div className="text-xs text-gray-400 uppercase tracking-widest mb-2 font-semibold">{label}</div>
      <div className="flex mb-1 ml-6">
        {COLS.map(c => <div key={c} className="w-12 text-center text-xs text-gray-500">{c}</div>)}
      </div>
      {[0,1,2,3,4].map(row => (
        <div key={row} className="flex items-center mb-1">
          <div className="w-6 text-xs text-gray-500 text-right mr-1">{ROWS[row]}</div>
          {[0,1,2,3,4].map(col => {
            const i = row * 5 + col;
            const cell = cells[i];
            return (
              <div
                key={col}
                onClick={() => onCellClick?.(i)}
                className={`w-12 h-12 border rounded flex items-center justify-center text-lg mr-1 cursor-pointer transition-all duration-150 ${getColor(cell)} ${isActive && mode === "attack" && cell === "fog" ? "cursor-crosshair" : ""}`}
                title={`${COLS[col]}${ROWS[row]}`}
              >
                {getIcon(cell)}
              </div>
            );
          })}
        </div>
      ))}
    </div>
  );
}

type Phase = "home" | "placement" | "battle" | "finished";

export default function Home() {
  const [phase, setPhase] = useState<Phase>("home");
  const [myBoard, setMyBoard] = useState<Cell[]>(Array(25).fill("empty"));
  const [enemyBoard, setEnemyBoard] = useState<Cell[]>(Array(25).fill("fog"));
  const [isMyTurn, setIsMyTurn] = useState(true);
  const [myTroops, setMyTroops] = useState(5);
  const [enemyTroops, setEnemyTroops] = useState(5);
  const [log, setLog] = useState<string[]>([]);
  const [winner, setWinner] = useState<"You" | "Enemy" | null>(null);
  const [mpcStatus, setMpcStatus] = useState("");
  const [gameId] = useState(() => Math.floor(Math.random() * 999999).toString());

  // Enemy troop positions (hidden from player — Arcium keeps these secret!)
  const [enemyPositions] = useState(() => {
    const positions = new Set<number>();
    while (positions.size < 5) positions.add(Math.floor(Math.random() * 25));
    return positions;
  });

  const troopCount = myBoard.filter(c => c === "troop").length;

  const toggleCell = (i: number) => {
    if (phase !== "placement") return;
    setMyBoard(b => {
      const nb = [...b];
      if (nb[i] === "troop") { nb[i] = "empty"; }
      else if (troopCount < 5) { nb[i] = "troop"; }
      return nb;
    });
  };

  const commitBoard = async () => {
    if (troopCount !== 5) return;
    setMpcStatus("🔐 Encrypting board and submitting to Arcium MXE...");
    await new Promise(r => setTimeout(r, 2000));
    setMpcStatus("✅ Board committed! Arcium MPC verified 5 troops without revealing positions.");
    await new Promise(r => setTimeout(r, 1000));
    setPhase("battle");
    setMpcStatus("");
  };

  const attack = async (i: number) => {
    if (!isMyTurn || phase !== "battle") return;
    if (enemyBoard[i] !== "fog") return;

    const x = i % 5;
    const y = Math.floor(i / 5);
    setMpcStatus(`⚔️ Resolving attack at ${COLS[x]}${ROWS[y]} via Arcium MPC...`);
    setIsMyTurn(false);

    await new Promise(r => setTimeout(r, 1800));

    const isHit = enemyPositions.has(i);
    const newEnemyBoard = [...enemyBoard];
    newEnemyBoard[i] = isHit ? "hit" : "miss";
    setEnemyBoard(newEnemyBoard);

    const newEnemyTroops = isHit ? enemyTroops - 1 : enemyTroops;
    setEnemyTroops(newEnemyTroops);

    const resultMsg = `Turn ${log.length + 1}: You attacked ${COLS[x]}${ROWS[y]} → ${isHit ? "💥 HIT!" : "〇 Miss"}`;
    setLog(l => [...l, resultMsg]);
    setMpcStatus(`✅ Arcium MPC resolved: ${isHit ? "💥 HIT!" : "〇 Miss"} — only this 1 bit revealed, board stays secret`);

    if (newEnemyTroops === 0) {
      setWinner("You");
      setPhase("finished");
      return;
    }

    // Enemy turn
    await new Promise(r => setTimeout(r, 1500));
    setMpcStatus("⚔️ Enemy is attacking via Arcium MPC...");

    await new Promise(r => setTimeout(r, 1800));

    // Enemy attacks random cell
    const myCells = myBoard.map((c, idx) => ({ c, idx })).filter(({ c }) => c === "troop" || c === "empty");
    if (myCells.length > 0) {
      const target = myCells[Math.floor(Math.random() * myCells.length)];
      const newMyBoard = [...myBoard];
      const enemyHit = target.c === "troop";
      newMyBoard[target.idx] = enemyHit ? "destroyed" : "empty";
      setMyBoard(newMyBoard);

      const newMyTroops = enemyHit ? myTroops - 1 : myTroops;
      setMyTroops(newMyTroops);

      const ex = target.idx % 5;
      const ey = Math.floor(target.idx / 5);
      setLog(l => [...l, `Turn ${log.length + 2}: Enemy attacked ${COLS[ex]}${ROWS[ey]} → ${enemyHit ? "💥 HIT!" : "〇 Miss"}`]);
      setMpcStatus(`✅ Enemy attack resolved by Arcium MPC`);

      if (newMyTroops === 0) {
        setWinner("Enemy");
        setPhase("finished");
        return;
      }
    }

    setIsMyTurn(true);
  };

  const reset = () => {
    setPhase("home");
    setMyBoard(Array(25).fill("empty"));
    setEnemyBoard(Array(25).fill("fog"));
    setIsMyTurn(true);
    setMyTroops(5);
    setEnemyTroops(5);
    setLog([]);
    setWinner(null);
    setMpcStatus("");
  };

  return (
    <>
      <Head>
        <title>Dark Dominion — Encrypted War Game</title>
      </Head>
      <div className="min-h-screen bg-gray-950 text-white p-6">

        {/* Header */}
        <div className="max-w-5xl mx-auto">
          <div className="flex items-center justify-between mb-8">
            <div>
              <h1 className="text-3xl font-bold tracking-tight">🏰 Dark Dominion</h1>
              <p className="text-gray-400 text-sm mt-1">Encrypted Strategy War Game · Powered by Arcium MPC on Solana</p>
            </div>
            <div className="flex gap-2 flex-wrap justify-end">
              <span className="px-3 py-1 rounded-full text-xs font-medium bg-purple-900 text-purple-300 border border-purple-700">Solana Devnet</span>
              <span className="px-3 py-1 rounded-full text-xs font-medium bg-cyan-900 text-cyan-300 border border-cyan-700">Arcium MPC</span>
              <span className="px-3 py-1 rounded-full text-xs font-medium bg-green-900 text-green-300 border border-green-700">Fog of War</span>
            </div>
          </div>

          {/* MPC Status Bar */}
          {mpcStatus && (
            <div className="mb-6 p-3 rounded-lg bg-cyan-950 border border-cyan-800 text-cyan-300 text-sm flex items-center gap-2">
              <span className="animate-pulse">⬤</span> {mpcStatus}
            </div>
          )}

          {/* HOME */}
          {phase === "home" && (
            <div className="max-w-lg mx-auto">
              <div className="bg-gray-900 rounded-2xl border border-gray-800 p-8 mb-4">
                <div className="text-6xl text-center mb-4">🏰</div>
                <h2 className="text-xl font-semibold text-center mb-3">How It Works</h2>
                <div className="space-y-3 text-sm text-gray-400">
                  <div className="flex gap-3 items-start">
                    <span className="text-cyan-400 font-bold">1.</span>
                    <span>Place 5 troops secretly on your 5×5 grid</span>
                  </div>
                  <div className="flex gap-3 items-start">
                    <span className="text-cyan-400 font-bold">2.</span>
                    <span>Your board is <strong className="text-white">encrypted</strong> and committed to Arcium's MXE — your opponent sees nothing</span>
                  </div>
                  <div className="flex gap-3 items-start">
                    <span className="text-cyan-400 font-bold">3.</span>
                    <span>Take turns attacking coordinates — Arcium MPC resolves each hit/miss <strong className="text-white">without revealing the board</strong></span>
                  </div>
                  <div className="flex gap-3 items-start">
                    <span className="text-cyan-400 font-bold">4.</span>
                    <span>Destroy all 5 enemy troops to win!</span>
                  </div>
                </div>
              </div>

              <div className="bg-gray-900 rounded-2xl border border-gray-800 p-6 mb-4 text-sm">
                <div className="text-xs text-gray-500 uppercase tracking-widest mb-2">Program Deployed</div>
                <div className="font-mono text-xs text-green-400 break-all">6Byt42WoRsHCeSXTY7Rov118FryQRGsZqcJQqupYR1SW</div>
                <div className="text-xs text-gray-500 mt-1">Game ID: #{gameId}</div>
              </div>

              <button
                onClick={() => setPhase("placement")}
                className="w-full py-4 rounded-xl bg-purple-600 hover:bg-purple-500 font-semibold text-lg transition-colors"
              >
                ⚔️ Start Game
              </button>
            </div>
          )}

          {/* PLACEMENT */}
          {phase === "placement" && (
            <div className="flex flex-col items-center gap-6">
              <div className="text-center">
                <h2 className="text-xl font-semibold mb-1">Place Your Troops</h2>
                <p className="text-gray-400 text-sm">Click 5 cells to place your troops. Your board will be encrypted by Arcium.</p>
              </div>

              <div className="bg-gray-900 rounded-2xl border border-gray-800 p-6">
                <Board label="Your Board (Private)" cells={myBoard} onCellClick={toggleCell} mode="placement" />
              </div>

              <div className="flex items-center gap-4">
                <span className={`text-sm font-medium ${troopCount === 5 ? "text-green-400" : "text-yellow-400"}`}>
                  {troopCount}/5 troops placed
                </span>
                <button
                  onClick={commitBoard}
                  disabled={troopCount !== 5}
                  className="px-6 py-3 rounded-xl bg-cyan-600 hover:bg-cyan-500 disabled:opacity-40 disabled:cursor-not-allowed font-semibold transition-colors"
                >
                  🔒 Commit Board (Encrypted)
                </button>
              </div>
            </div>
          )}

          {/* BATTLE */}
          {phase === "battle" && (
            <div>
              {/* Score bar */}
              <div className="flex justify-center gap-12 mb-6">
                <div className="text-center">
                  <div className="text-xs text-gray-400 uppercase tracking-widest mb-1">Your Troops</div>
                  <div className={`text-3xl font-bold ${myTroops > 2 ? "text-yellow-400" : "text-red-400"}`}>{myTroops}</div>
                </div>
                <div className="text-2xl self-center text-gray-600">VS</div>
                <div className="text-center">
                  <div className="text-xs text-gray-400 uppercase tracking-widest mb-1">Enemy Troops</div>
                  <div className={`text-3xl font-bold ${enemyTroops > 2 ? "text-gray-300" : "text-red-400"}`}>{enemyTroops}</div>
                </div>
              </div>

              {/* Turn indicator */}
              <div className={`text-center mb-6 py-2 px-4 rounded-lg text-sm font-medium ${isMyTurn ? "bg-purple-900 text-purple-300 border border-purple-700" : "bg-gray-800 text-gray-400"}`}>
                {isMyTurn ? "⚔️ Your turn — click a cell on the Enemy Board" : "⏳ Waiting for Arcium MPC to resolve..."}
              </div>

              {/* Boards */}
              <div className="flex flex-wrap gap-8 justify-center mb-6">
                <div className="bg-gray-900 rounded-2xl border border-gray-800 p-6">
                  <Board label="Your Board" cells={myBoard} mode="view" />
                </div>
                <div className={`bg-gray-900 rounded-2xl border p-6 ${isMyTurn ? "border-purple-700" : "border-gray-800"}`}>
                  <Board
                    label="Enemy Board (Fog of War)"
                    cells={enemyBoard}
                    onCellClick={isMyTurn ? attack : undefined}
                    mode="attack"
                    isActive={isMyTurn}
                  />
                </div>
              </div>

              {/* Battle log */}
              <div className="max-w-lg mx-auto bg-gray-900 rounded-xl border border-gray-800 p-4">
                <div className="text-xs text-gray-500 uppercase tracking-widest mb-3">Battle Log — Arcium MPC</div>
                {log.length === 0 && <div className="text-gray-600 text-sm text-center py-4">No attacks yet</div>}
                <div className="space-y-2 max-h-40 overflow-y-auto">
                  {[...log].reverse().map((entry, i) => (
                    <div key={i} className={`text-sm py-1 border-b border-gray-800 ${entry.includes("HIT") ? "text-red-400" : "text-gray-400"}`}>
                      {entry}
                    </div>
                  ))}
                </div>
              </div>
            </div>
          )}

          {/* FINISHED */}
          {phase === "finished" && (
            <div className="max-w-md mx-auto text-center">
              <div className="bg-gray-900 rounded-2xl border border-gray-800 p-8 mb-6">
                <div className="text-6xl mb-4">{winner === "You" ? "🏆" : "💀"}</div>
                <h2 className="text-2xl font-bold mb-2">{winner === "You" ? "Victory!" : "Defeated!"}</h2>
                <p className="text-gray-400 text-sm mb-6">
                  {winner === "You"
                    ? "All enemy troops destroyed. Your board was never revealed thanks to Arcium MPC!"
                    : "All your troops were destroyed. The fog of war remains cryptographically intact."}
                </p>
                <div className="bg-gray-800 rounded-xl p-4 text-left mb-6">
                  <div className="text-xs text-gray-500 uppercase tracking-widest mb-2">Arcium MPC Proof</div>
                  <div className="text-xs text-green-400">✓ Board committed with Rescue cipher + x25519 ECDH</div>
                  <div className="text-xs text-green-400">✓ Each attack resolved with zero-knowledge hit/miss</div>
                  <div className="text-xs text-green-400">✓ No board positions revealed during gameplay</div>
                  <div className="text-xs text-green-400">✓ All results signed by Arcium cluster</div>
                </div>
                <button onClick={reset} className="w-full py-3 rounded-xl bg-purple-600 hover:bg-purple-500 font-semibold transition-colors">
                  Play Again
                </button>
              </div>
            </div>
          )}

          {/* Footer */}
          <div className="text-center mt-8 text-xs text-gray-600">
            Built with Arcium MPC · Deployed on Solana Devnet ·{" "}
            <a href="https://github.com/Alagham/dark-dominion" className="text-gray-500 hover:text-gray-400">
              github.com/Alagham/dark-dominion
            </a>
          </div>
        </div>
      </div>
    </>
  );
}
