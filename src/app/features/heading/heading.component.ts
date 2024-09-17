import { Component, Input, OnInit } from '@angular/core';
import { HashHoverFeature } from '../../services/ftHashHover.service';
import { CommonModule } from '@angular/common';
import { content } from '../../docs/content/content';
import { LanguageService } from '../../services/navLanguage.service';
import { Subscription } from 'rxjs';

@Component({
  selector: 'heading',
  standalone: true,
  imports: [
    CommonModule
  ],
  templateUrl: './heading.component.html',
  styleUrl: './heading.component.css'
})
export class HeadingComponent {
  @Input() path!: [string, string, string];
  @Input() article!: string;
  @Input() write!: string;
  @Input() subtitleClass: string = 'docs-subtitle';
  @Input() hovering: boolean = true;

  constructor(
    public hashHoverFeature: HashHoverFeature,
    private languageService: LanguageService,
  ) { }

  writer: string = '';

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
    this.writer = content[lang][title][page][article][write];
  }

  private concatArticle(article: string): string {
    return `article_${article}`;
  }
}
