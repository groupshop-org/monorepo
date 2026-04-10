pub struct PageContent {
    pub title: &'static str,
    pub html: &'static str,
}

pub fn privacy_policy() -> PageContent {
    PageContent {
        title: "Privacy Policy",
        html: r#"
            <p><strong>Effective Date:</strong> April 10, 2026</p>
            <p>
                This Privacy Policy explains how Groupshop collects, uses, shares, and protects personal information when you use the Groupshop website, applications, and related services, including our on-chain group buying features, account features, and customer support.
            </p>
            <p>
                Groupshop is building a marketplace for real-world group buying secured through on-chain coordination. Because portions of the service may interact with public blockchains, some information associated with your use of the service may be publicly visible and cannot be altered or deleted by Groupshop.
            </p>

            <h2>1. Scope</h2>
            <p>
                This Privacy Policy applies to information we collect when you visit our landing site, create or use a Groupshop account, join a product deal, connect a wallet, contact us, or otherwise interact with our services.
            </p>

            <h2>2. Information We Collect</h2>
            <h3>Information you provide directly</h3>
            <ul>
                <li>Name, username, email address, shipping address, billing details, and other account information.</li>
                <li>Order, deal participation, refund, and fulfillment information.</li>
                <li>Communications you send to us, including support requests and survey responses.</li>
            </ul>

            <h3>Information collected automatically</h3>
            <ul>
                <li>Device, browser, IP address, approximate location, and log information.</li>
                <li>Usage data, such as pages viewed, links clicked, referral URLs, and timestamps.</li>
                <li>Cookie, local storage, and similar technical data used to operate and secure the service.</li>
            </ul>

            <h3>Blockchain and wallet information</h3>
            <ul>
                <li>Public wallet addresses.</li>
                <li>Transaction hashes, amounts, timestamps, and related on-chain metadata.</li>
                <li>Public blockchain state associated with your participation in a deal.</li>
            </ul>
            <p>
                Public blockchain data is not controlled by Groupshop. Transactions recorded on Solana or any other public blockchain may remain publicly accessible even if you stop using our services.
            </p>

            <h3>Information from third parties</h3>
            <ul>
                <li>Identity and login information from authentication providers.</li>
                <li>Payment, treasury, compliance, logistics, shipping, fraud-prevention, and fulfillment data from vendors and service providers.</li>
                <li>Publicly available blockchain data and analytics derived from it.</li>
            </ul>

            <h2>3. How We Use Information</h2>
            <ul>
                <li>Provide, operate, maintain, and improve Groupshop.</li>
                <li>Create and manage accounts, authenticate users, and secure the service.</li>
                <li>Process deal participation, escrow-related workflows, refunds, fulfillment, shipping, and support requests.</li>
                <li>Prevent fraud, abuse, unauthorized access, and other harmful activity.</li>
                <li>Comply with legal obligations, enforce our terms, and protect rights, safety, and property.</li>
                <li>Communicate with you about your account, transactions, updates, and service-related notices.</li>
                <li>Send marketing communications where permitted by law and subject to your preferences.</li>
            </ul>

            <h2>4. How We Share Information</h2>
            <p>We may share information with:</p>
            <ul>
                <li>Service providers that host, secure, analyze, fulfill, support, or operate the service.</li>
                <li>Suppliers, merchants, logistics partners, and fulfillment providers involved in completing purchases and deliveries.</li>
                <li>Payment, treasury, compliance, identity, fraud, and wallet infrastructure partners.</li>
                <li>Law enforcement, regulators, courts, or other third parties when required by law or reasonably necessary to protect the service or others.</li>
                <li>Successors or counterparties involved in a merger, acquisition, financing, asset sale, or similar corporate transaction.</li>
                <li>Other parties when you direct us to share information or otherwise consent.</li>
            </ul>
            <p>
                We may also share information that is already public on a blockchain or information that has been aggregated or de-identified so that it cannot reasonably be used to identify you.
            </p>

            <h2>5. Cookies and Similar Technologies</h2>
            <p>
                We use cookies, local storage, and similar technologies to remember preferences, maintain sessions, analyze service performance, and protect the service. You can adjust browser settings to control cookies, but some features may not function properly if you disable them.
            </p>

            <h2>6. Retention</h2>
            <p>
                We retain personal information for as long as reasonably necessary to provide the service, complete transactions, comply with law, resolve disputes, enforce agreements, and maintain security and business records. Public blockchain data may persist indefinitely outside our systems.
            </p>

            <h2>7. Security</h2>
            <p>
                We use reasonable administrative, technical, and physical safeguards designed to protect personal information. No method of transmission or storage is completely secure, and we cannot guarantee absolute security.
            </p>

            <h2>8. Your Rights and Choices</h2>
            <p>
                Depending on your location, you may have rights to request access to, correction of, deletion of, or portability of certain personal information, or to object to or limit certain processing. You may also opt out of marketing emails by using the unsubscribe mechanism in those messages.
            </p>
            <p>
                Because blockchain records are public and immutable, Groupshop may not be able to modify or delete on-chain information even if we can update or delete associated off-chain records.
            </p>

            <h2>9. California Privacy Notice</h2>
            <p>
                If you are a California resident, you may have rights under California privacy law, including the right to know about categories of personal information we collect, use, disclose, or retain; the right to request deletion or correction of certain personal information; and the right not to be discriminated against for exercising your rights. We do not sell personal information or share personal information for cross-context behavioral advertising as those terms are used in California law.
            </p>
            <p>
                In the previous 12 months, the categories of personal information we may have collected or disclosed for business purposes include identifiers, customer records information, commercial information, internet or network activity, approximate geolocation, user-generated communications, and inferences used to operate and protect the service.
            </p>

            <h2>10. International Transfers</h2>
            <p>
                Groupshop may process and store information in the United States and other countries where we or our service providers operate. Those locations may have different data protection laws than your jurisdiction.
            </p>

            <h2>11. Children’s Privacy</h2>
            <p>
                Groupshop is not directed to children under 13, and we do not knowingly collect personal information from children under 13. If you believe a child has provided personal information to us, please contact us so we can investigate and take appropriate steps.
            </p>

            <h2>12. Changes to This Privacy Policy</h2>
            <p>
                We may update this Privacy Policy from time to time. If we make material changes, we will post the updated version here and update the Effective Date above. Your continued use of the service after the updated policy becomes effective is subject to the revised policy.
            </p>

            <h2>13. Contact Us</h2>
            <p>
                For privacy requests or questions about this Privacy Policy, contact us at <a href="mailto:contract@groupshop.org">contract@groupshop.org</a>.
            </p>
        "#,
    }
}

pub fn terms_of_service() -> PageContent {
    PageContent {
        title: "Terms of Service",
        html: r#"
            <p><strong>Effective Date:</strong> April 10, 2026</p>
            <p>
                These Terms of Service govern your access to and use of the Groupshop website, applications, and related services, including any group buying, account, wallet, order, shipping, and support features that we make available.
            </p>
            <p>
                By accessing or using Groupshop, you agree to these Terms. If you do not agree, do not use the service.
            </p>

            <h2>1. Eligibility</h2>
            <p>
                You must be at least the age of majority in your jurisdiction and capable of forming a binding agreement to use Groupshop. If you use the service on behalf of a business or other entity, you represent that you have authority to bind that entity to these Terms.
            </p>

            <h2>2. Your Account</h2>
            <p>
                You are responsible for maintaining the confidentiality of your account credentials, keeping your information accurate, and all activity that occurs under your account. You must notify us promptly if you believe your account has been compromised.
            </p>

            <h2>3. Description of the Service</h2>
            <p>
                Groupshop provides a platform for coordinated group purchasing of real-world products. We may offer product listings, group deal thresholds, wallet-based participation flows, escrow-related coordination, order management, fulfillment updates, and customer support.
            </p>
            <p>
                Features may change over time, may not be available in all locations, and may be subject to additional product-specific disclosures presented at the point of use.
            </p>

            <h2>4. Blockchain and Wallet Terms</h2>
            <ul>
                <li>You are solely responsible for your wallet, wallet credentials, private keys, and recovery phrases.</li>
                <li>Blockchain transactions may be irreversible, may fail, and may involve network fees or validator fees beyond our control.</li>
                <li>Public blockchain activity, including wallet addresses and transaction metadata, may be visible to others.</li>
                <li>Groupshop does not guarantee the performance, availability, or security of any blockchain network, wallet provider, validator, or third-party protocol.</li>
            </ul>

            <h2>5. Orders, Group Deals, Pricing, and Refunds</h2>
            <ul>
                <li>Deals may be subject to minimum participation thresholds, deadlines, inventory limits, pricing conditions, geographic restrictions, and supplier acceptance.</li>
                <li>Product availability, shipping costs, tax treatment, and final fulfillment timing may change before an order is finalized.</li>
                <li>We may cancel, suspend, or refuse a deal or order if required for legal, operational, fraud, safety, inventory, pricing, or supplier reasons.</li>
                <li>If a deal fails to close or cannot be fulfilled, we may issue a refund, reversal, or other remediation as required by applicable law and our checkout disclosures.</li>
                <li>Where applicable, title, risk of loss, return rights, or warranty coverage may be subject to merchant, supplier, or carrier terms in addition to these Terms.</li>
            </ul>

            <h2>6. Prohibited Conduct</h2>
            <p>You may not:</p>
            <ul>
                <li>Use Groupshop in violation of law or regulation.</li>
                <li>Interfere with the security, integrity, or operation of the service.</li>
                <li>Access the service using automated means except as expressly authorized.</li>
                <li>Misrepresent your identity, wallet ownership, eligibility, or authority.</li>
                <li>Use the service for fraud, abuse, money laundering, sanctions evasion, or other unlawful conduct.</li>
                <li>Reverse engineer, scrape, or exploit the service except as permitted by law and these Terms.</li>
            </ul>

            <h2>7. Intellectual Property</h2>
            <p>
                Groupshop and its licensors retain all rights, title, and interest in the service, including software, content, branding, design, and related intellectual property, except for content or materials owned by third parties. Subject to these Terms, we grant you a limited, revocable, non-exclusive, non-transferable right to access and use the service for its intended purpose.
            </p>

            <h2>8. Feedback</h2>
            <p>
                If you provide feedback, suggestions, or ideas about Groupshop, you grant us a worldwide, royalty-free, perpetual, irrevocable license to use, modify, and incorporate that feedback without restriction or compensation.
            </p>

            <h2>9. Third-Party Services</h2>
            <p>
                Groupshop may rely on or link to third-party services, including wallet providers, authentication providers, suppliers, logistics providers, fulfillment partners, and payment or treasury partners. We are not responsible for third-party services, and your use of them may be governed by separate terms and privacy policies.
            </p>

            <h2>10. Disclaimers</h2>
            <p>
                To the fullest extent permitted by law, the service is provided “as is” and “as available,” without warranties of any kind, whether express, implied, or statutory, including warranties of merchantability, fitness for a particular purpose, title, non-infringement, or that the service will be uninterrupted, secure, or error-free.
            </p>
            <p>
                Groupshop does not provide legal, tax, investment, financial, or other professional advice. You are solely responsible for evaluating any blockchain, wallet, product, tax, or compliance consequences of your use of the service.
            </p>

            <h2>11. Limitation of Liability</h2>
            <p>
                To the fullest extent permitted by law, Groupshop and its affiliates, officers, directors, employees, contractors, and licensors will not be liable for any indirect, incidental, special, consequential, exemplary, or punitive damages, or any loss of profits, revenue, goodwill, data, use, or business opportunity, arising out of or related to the service, even if advised of the possibility of those damages.
            </p>
            <p>
                To the fullest extent permitted by law, our aggregate liability for claims arising out of or relating to the service will not exceed the greater of (a) the amount you paid to Groupshop for the specific transaction giving rise to the claim in the 12 months preceding the event giving rise to the claim or (b) USD $100.
            </p>

            <h2>12. Indemnity</h2>
            <p>
                You agree to defend, indemnify, and hold harmless Groupshop and its affiliates, officers, directors, employees, contractors, and licensors from and against claims, liabilities, damages, judgments, losses, costs, and expenses, including reasonable attorneys’ fees, arising out of or related to your use of the service, your violation of these Terms, or your violation of law or third-party rights.
            </p>

            <h2>13. Suspension and Termination</h2>
            <p>
                We may suspend or terminate your access to all or part of the service at any time, with or without notice, if we reasonably believe you have violated these Terms, created risk for the service or others, or if suspension or termination is otherwise required for legal, security, or operational reasons.
            </p>

            <h2>14. Changes to the Service or Terms</h2>
            <p>
                We may change, suspend, or discontinue any part of the service at any time. We may also update these Terms from time to time by posting the revised version and updating the Effective Date above. Your continued use of the service after the updated Terms become effective constitutes acceptance of the revised Terms.
            </p>

            <h2>15. General</h2>
            <p>
                If any provision of these Terms is held unenforceable, the remaining provisions will remain in effect. Our failure to enforce any provision is not a waiver of that provision. These Terms, together with any additional terms presented for specific features, form the entire agreement between you and Groupshop regarding the service, except to the extent a separate written agreement applies.
            </p>

            <h2>16. Contact Us</h2>
            <p>
                Questions about these Terms may be sent to <a href="mailto:contract@groupshop.org">contract@groupshop.org</a>.
            </p>
        "#,
    }
}
