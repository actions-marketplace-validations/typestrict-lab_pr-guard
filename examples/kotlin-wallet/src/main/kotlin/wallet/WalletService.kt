package wallet

import java.math.RoundingMode

class WalletService(private val rates: ExchangeRates) {
    private val wallets = mutableMapOf<String, Wallet>()
    private val pins = mutableMapOf<String, String>()

    fun open(id: String, ownerId: String, currency: String, pin: String): Wallet {
        require(pin.length in 4..8 && pin.all(Char::isDigit)) { "PIN must have 4 to 8 digits" }
        require(id !in wallets) { "wallet $id already exists" }
        val wallet = Wallet(id, ownerId, 0, currency)
        wallets[id] = wallet
        pins[id] = PinHasher.hash(pin)
        return wallet
    }

    fun get(id: String): Wallet = wallets[id] ?: throw NoSuchElementException("unknown wallet $id")

    fun deposit(id: String, amountCents: Long): Wallet {
        require(amountCents > 0) { "deposit must be positive" }
        val wallet = get(id)
        return wallet.copy(balanceCents = wallet.balanceCents + amountCents).also { wallets[id] = it }
    }

    /** Moves money between wallets, converting currency when they differ. Returns the amount credited. */
    fun transfer(callerId: String, fromId: String, toId: String, amountCents: Long, pin: String): Long {
        require(amountCents > 0) { "transfer must be positive" }
        require(fromId != toId) { "cannot transfer to the same wallet" }
        val from = get(fromId)
        val to = get(toId)
        if (from.ownerId != callerId) throw Forbidden("caller does not own wallet $fromId")
        if (!PinHasher.verify(pin, pins.getValue(fromId))) throw Forbidden("wrong PIN")
        if (from.balanceCents < amountCents) throw InsufficientFunds(fromId)

        val credited = convert(amountCents, from.currency, to.currency)
        wallets[fromId] = from.copy(balanceCents = from.balanceCents - amountCents)
        wallets[toId] = to.copy(balanceCents = to.balanceCents + credited)
        return credited
    }

    private fun convert(amountCents: Long, from: String, to: String): Long {
        if (from == to) return amountCents
        return amountCents.toBigDecimal()
            .multiply(rates.rate(from, to))
            .setScale(0, RoundingMode.HALF_EVEN)
            .longValueExact()
    }
}
