import { CommonModule } from '@angular/common';
import { Component, Input } from '@angular/core';
import { content } from '../../docs/content/content';
import { LanguageService } from '../../services/navLanguage.service';
import { Subscription } from 'rxjs';
import { RouterLink } from '@angular/router';

@Component({
  selector: 'anchor',
  standalone: true,
  imports: [
    CommonModule,
    RouterLink
  ],
  templateUrl: './anchor.component.html',
  styleUrl: './anchor.component.css'
})
export class AnchorComponent {
  @Input() path!: [string, string, string];
  @Input() article!: string;
  @Input() write!: string;
  @Input() available: boolean = true;
  @Input() subItem?: string;

  constructor(
    private languageService: LanguageService,
  ) { }

  writer: string[] = [];
  anchorLink: string = '';
  anchorFragmentLink: string = '';

  get anchorClass(): string {
    return this.available ? '' : 'not-available';
  }
  ngOnInit(): void {
    this.article = this.concatArticle(this.article);
    this.subscribeToLanguageChanges();
    this.resolveContent();
  }

  languageSubscription!: Subscription;
  private subscribeToLanguageChanges(): void {
    this.languageSubscription = this.languageService.currentLanguage$.subscribe((Lang) => {
      this.path[0] = Lang;
      this.resolveContent();
    });
  }

  private resolveContent(): void {
    const [lang, title, page] = this.path;
    const article = this.article;
    const write = this.write;
    const contentItem = content[lang][title][page][article];
    // Resolve Logic
    if (this.subItem) {
      // For nested Anchors
      const subItemContent = contentItem[this.subItem];
      this.writer = subItemContent[write][1][0];
      this.anchorLink = subItemContent[write][1][1];
      this.anchorFragmentLink = subItemContent[write][1][2];
    } else {
      // For normal Anchors
      this.writer = contentItem[write][0];
      this.anchorLink = contentItem[write][1];
      this.anchorFragmentLink = contentItem[write][2];
    }
  }

  private concatArticle(article: string): string {
    return `article_${article}`;
  }

}
