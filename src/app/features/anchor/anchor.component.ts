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

  constructor(
    private languageService: LanguageService,
  ) { }

  writer: string = '';
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
    this.writer = content[lang][title][page][article][write].c;
    this.anchorLink = content[lang][title][page][article][write].link;
    this.anchorFragmentLink = content[lang][title][page][article][write].fragmentLink;
    // console.log(`
    //   a Paragraph: ${this.writer}
    //   a path: ${this.path}
    //   a article: ${this.article}`);
  }

  private concatArticle(article: string): string {
    return `article_${article}`;
  }

}
