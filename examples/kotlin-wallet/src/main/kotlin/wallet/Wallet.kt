package wallet

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class Wallet(
    val id: String,
    @SerialName("owner_id") val ownerId: String,
    /** Balance in minor units (cents). */
    @SerialName("balance") val balanceCents: Long,
    val currency: String,
)

class InsufficientFunds(walletId: String) : RuntimeException("wallet $walletId has insufficient funds")

class Forbidden(message: String) : RuntimeException(message)
