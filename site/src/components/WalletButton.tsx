import { Wallet } from "lucide-react";
import type { WalletState } from "../types";
import { compactAddress } from "../lib/format";

type Props = {
  wallet: WalletState;
  onConnect: () => void;
  busy: boolean;
};

export function WalletButton({ wallet, onConnect, busy }: Props) {
  return (
    <button className="wallet-button" onClick={onConnect} disabled={busy}>
      <Wallet size={18} />
      {wallet.connected ? compactAddress(wallet.publicKey) : busy ? "Connecting" : "Connect wallet"}
    </button>
  );
}
